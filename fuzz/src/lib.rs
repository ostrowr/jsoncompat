mod format_gen;
mod regex_gen;

use fancy_regex::Regex;
use json_schema_ast::{
    ArrayContains, IntegerMultipleOf, ResolvedNode, ResolvedNodeKind, ResolvedSchema,
    SchemaBuildError,
};
use rand::{Rng, RngExt};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::num::NonZeroUsize;

const DEFAULT_MAX_GENERATION_ATTEMPTS: usize = 100;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GenerateError {
    #[error(transparent)]
    Schema(#[from] SchemaBuildError),
    #[error("failed to generate a value accepted by the raw schema after {attempts} attempts")]
    ExhaustedAttempts { attempts: NonZeroUsize },
}

/// Stateful value generator for resolved schema graphs.
#[derive(Debug)]
pub struct ValueGenerator {
    max_generation_attempts: NonZeroUsize,
}

impl Default for ValueGenerator {
    fn default() -> Self {
        Self {
            max_generation_attempts: NonZeroUsize::new(DEFAULT_MAX_GENERATION_ATTEMPTS)
                .expect("default generation attempt limit must be non-zero"),
        }
    }
}

#[derive(Clone, Copy)]
struct ArrayGenerationSchema<'a> {
    prefix_items: &'a [ResolvedNode],
    items: &'a ResolvedNode,
    contains: Option<&'a ArrayContains<ResolvedNode>>,
    unique_items: bool,
}

impl ValueGenerator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_max_generation_attempts(max_generation_attempts: NonZeroUsize) -> Self {
        Self {
            max_generation_attempts,
        }
    }

    /// Generate a random JSON value that satisfies `schema` according to the
    /// raw-schema validator backend.
    ///
    /// Internally this walks the canonicalized `ResolvedNode` graph to produce
    /// candidate values, but only returns once `ResolvedSchema::is_valid()`
    /// accepts the candidate.
    /// We limit recursion with `depth`.
    pub fn generate_value(
        &mut self,
        schema: &ResolvedSchema,
        rng: &mut impl Rng,
        depth: u8,
    ) -> Result<Value, GenerateError> {
        let root = schema.root()?;
        for _ in 0..self.max_generation_attempts.get() {
            let candidate = self.generate_candidate(root, rng, depth);
            if schema.is_valid(&candidate)? {
                return Ok(candidate);
            }
        }

        Err(GenerateError::ExhaustedAttempts {
            attempts: self.max_generation_attempts,
        })
    }

    fn generate_candidate(
        &mut self,
        schema: &ResolvedNode,
        rng: &mut impl Rng,
        depth: u8,
    ) -> Value {
        generate_candidate_with_context(schema, rng, depth, self)
    }
    fn schema_accepts_value(&mut self, schema: &ResolvedNode, value: &Value) -> bool {
        schema.accepts_value(value)
    }

    fn generate_property_value(
        &mut self,
        property_name: &str,
        property_schema: &ResolvedNode,
        pattern_properties: &HashMap<String, ResolvedNode>,
        rng: &mut impl Rng,
        depth: u8,
    ) -> Value {
        if !property_matches_any_pattern(pattern_properties, property_name) {
            return self.generate_candidate(property_schema, rng, depth);
        }

        for _ in 0..32 {
            let candidate = self.generate_candidate(property_schema, rng, depth);
            if pattern_property_schemas_accept_value(
                self,
                pattern_properties,
                property_name,
                &candidate,
            ) {
                return candidate;
            }
        }

        for pattern_schema in matching_pattern_property_schemas(pattern_properties, property_name) {
            let candidate = self.generate_candidate(pattern_schema, rng, depth);
            if self.schema_accepts_value(property_schema, &candidate)
                && pattern_property_schemas_accept_value(
                    self,
                    pattern_properties,
                    property_name,
                    &candidate,
                )
            {
                return candidate;
            }
        }

        self.generate_candidate(property_schema, rng, depth)
    }

    fn generate_additional_property_value(
        &mut self,
        property_name: &str,
        pattern_properties: &HashMap<String, ResolvedNode>,
        additional: &ResolvedNode,
        rng: &mut impl Rng,
        depth: u8,
    ) -> Value {
        let matching_schemas = matching_pattern_property_schemas(pattern_properties, property_name);
        let Some(first_pattern_schema) = matching_schemas.first() else {
            return self.generate_candidate(additional, rng, depth);
        };

        for _ in 0..32 {
            let candidate = self.generate_candidate(first_pattern_schema, rng, depth);
            if matching_schemas
                .iter()
                .all(|schema| self.schema_accepts_value(schema, &candidate))
            {
                return candidate;
            }
        }

        self.generate_candidate(first_pattern_schema, rng, depth)
    }
}

/// Generate a random JSON value that satisfies `schema` according to the
/// raw-schema validator backend.
/// We limit recursion with `depth`.
pub fn generate_value(
    schema: &ResolvedSchema,
    rng: &mut impl Rng,
    depth: u8,
) -> Result<Value, GenerateError> {
    ValueGenerator::new().generate_value(schema, rng, depth)
}

fn generate_candidate_with_context(
    schema: &ResolvedNode,
    rng: &mut impl Rng,
    depth: u8,
    generator: &mut ValueGenerator,
) -> Value {
    if depth == 0 {
        return Value::Null;
    }

    use ResolvedNodeKind::*;

    match schema.kind() {
        BoolSchema(false) => Value::Null,
        BoolSchema(true) | Any => random_any(rng, depth),

        Enum(vals) if !vals.is_empty() => {
            let idx = rng.random_range(0..vals.len());
            vals[idx].clone()
        }

        AllOf(subs) if !subs.is_empty() => {
            // Heuristic strategy: try to satisfy *all* branches while keeping the
            // generation cheap.  The original implementation relied on a long
            // explanatory comment; reintroduce the gist:
            //   1. bail out if any subschema is `false` (the intersection is empty),
            //   2. when all branches describe objects, merge their generated
            //      property maps so the final instance satisfies every branch,
            //   3. otherwise fall back to any non-trivial branch (ignoring
            //      `true`/`Any`) which is usually sufficient for simple
            //      intersections such as `[{}, {"type": "number"}]`.
            if subs.iter().any(|s| matches!(s.kind(), BoolSchema(false))) {
                return Value::Null;
            }

            let non_trivial = subs
                .iter()
                .filter(|sub| !matches!(sub.kind(), BoolSchema(true) | Any))
                .collect::<Vec<_>>();

            let object_subschemas = subs
                .iter()
                .map(object_schema_branch)
                .collect::<Option<Vec<_>>>();

            for _ in 0..32 {
                let candidate = if let Some(object_subschemas) = &object_subschemas {
                    use std::collections::HashMap;

                    let mut combined = Map::new();
                    for sub in object_subschemas {
                        if let Object {
                            properties,
                            pattern_properties,
                            required,
                            additional,
                            min_properties,
                            ..
                        } = sub.kind()
                            && let Value::Object(obj) =
                                generator.generate_candidate(sub, rng, depth.saturating_sub(1))
                        {
                            for (k, v) in obj {
                                if required.contains(&k)
                                    || !matches!(additional.kind(), BoolSchema(true) | Any)
                                    || properties.contains_key(&k)
                                    || property_matches_any_pattern(pattern_properties, &k)
                                    || min_properties
                                        .is_some_and(|minimum| minimum > required.len())
                                {
                                    combined.insert(k, v);
                                }
                            }
                        }
                    }

                    // Ensure that every branch's required properties exist in the
                    // merged object.  The generator is probabilistic, so it is
                    // possible that the fast generation above skipped an optional
                    // property that later turned out to be required by another
                    // branch.  We deterministically fill any such gaps here.
                    let mut missing: HashMap<std::string::String, Option<ResolvedNode>> =
                        HashMap::new();
                    for sub in object_subschemas {
                        if let Object {
                            properties,
                            required,
                            ..
                        } = sub.kind()
                        {
                            for req in required {
                                if !combined.contains_key(req) {
                                    missing.insert(req.clone(), properties.get(req).cloned());
                                }
                            }
                        }
                    }

                    for (k, schema) in missing {
                        let val = match schema {
                            Some(schema) => {
                                generator.generate_candidate(&schema, rng, depth.saturating_sub(1))
                            }
                            None => random_any(rng, depth.saturating_sub(1)),
                        };
                        combined.insert(k, val);
                    }

                    Value::Object(combined)
                } else if non_trivial.is_empty() {
                    random_any(rng, depth)
                } else {
                    let index = rng.random_range(0..non_trivial.len());
                    generator.generate_candidate(non_trivial[index], rng, depth.saturating_sub(1))
                };

                if generator.schema_accepts_value(schema, &candidate) {
                    return candidate;
                }
            }

            random_any(rng, depth)
        }

        AnyOf(subs) if !subs.is_empty() => {
            for _ in 0..32 {
                let index = rng.random_range(0..subs.len());
                let candidate =
                    generator.generate_candidate(&subs[index], rng, depth.saturating_sub(1));
                if generator.schema_accepts_value(&subs[index], &candidate) {
                    return candidate;
                }
            }

            random_any(rng, depth)
        }

        OneOf(subs) if !subs.is_empty() => {
            for _ in 0..32 {
                let pick = rng.random_range(0..subs.len());
                let candidate_schema =
                    object_schema_branch(&subs[pick]).unwrap_or_else(|| subs[pick].clone());
                let candidate =
                    generator.generate_candidate(&candidate_schema, rng, depth.saturating_sub(1));

                let mut ok = 0;
                for child in subs {
                    if generator.schema_accepts_value(child, &candidate) {
                        ok += 1;
                        if ok > 1 {
                            break;
                        }
                    }
                }

                if ok == 1 {
                    return candidate;
                }
            }

            random_any(rng, depth)
        }

        Not(negated) => generate_not_value(schema, negated, rng, depth, generator),

        String {
            min_length,
            max_length,
            pattern,
            format,
            enumeration,
        } => {
            if let Some(e) = enumeration
                && !e.is_empty()
            {
                let idx = rng.random_range(0..e.len());
                return e[idx].clone();
            }
            if let Some(pattern) = pattern
                && let Some(candidate) = literal_from_simple_pattern(pattern)
                && candidate.len() as u64 >= min_length.unwrap_or(0)
                && max_length.is_none_or(|limit| (candidate.len() as u64) <= limit)
            {
                return Value::String(candidate);
            }

            if let Some(fmt) = format
                && let Some(s) = format_gen::generate_for_format(fmt, rng)
            {
                // min_length/max_length constraints are ignored when format is used.
                return Value::String(s);
            }

            if let Some(pat) = pattern
                && let Some(s) = regex_gen::generate_matching_string(pat, rng)
            {
                // min_length/max_length constraints are ignored when pattern is used.
                return Value::String(s);
            }

            let len_min = min_length.unwrap_or(0);
            let len_max = max_length.unwrap_or(len_min + 5).max(len_min);
            let length = if len_min <= len_max {
                rng.random_range(len_min..=len_max.min(len_min + 10))
            } else {
                len_min
            };
            let s: std::string::String = (0..length)
                .map(|_| rng.sample(rand::distr::Alphanumeric) as char)
                .collect();
            Value::String(s)
        }

        Number {
            enumeration,
            minimum,
            maximum,
            multiple_of,
            ..
        } => {
            if let Some(e) = enumeration
                && !e.is_empty()
            {
                let idx = rng.random_range(0..e.len());
                return e[idx].clone();
            }
            if multiple_of.is_some() {
                let low = minimum.unwrap_or(f64::NEG_INFINITY);
                let high = maximum.unwrap_or(f64::INFINITY);
                if low <= 0.0 && 0.0 <= high {
                    return Value::Number(serde_json::Number::from_f64(0.0).unwrap());
                }
            }

            let low = minimum.unwrap_or(0.0).max(-1_000_000.0);
            let high = maximum.unwrap_or(1_000_000.0).min(1_000_000.0);
            let mut val = rng.random_range(low..=high);

            if let Some(mo) = multiple_of
                && *mo > 0.0
            {
                let k = (val / *mo).floor();
                val = k * *mo;
                if val < low || val > high {
                    let k = (low / *mo).ceil();
                    val = k * *mo;
                }
            }

            Value::Number(serde_json::Number::from_f64(val).unwrap_or_else(|| 0.into()))
        }

        Integer {
            enumeration,
            minimum,
            maximum,
            multiple_of,
            ..
        } => {
            if let Some(e) = enumeration
                && !e.is_empty()
            {
                let idx = rng.random_range(0..e.len());
                return e[idx].clone();
            }
            if multiple_of.is_some() {
                let low = minimum.unwrap_or(i64::MIN);
                let high = maximum.unwrap_or(i64::MAX);
                if low <= 0 && 0 <= high {
                    return Value::Number(0.into());
                }
            }

            let low = minimum.unwrap_or(-1000).max(-1_000_000);
            let high = maximum.unwrap_or(1000).min(1_000_000);
            let mut val = rng.random_range(low..=high);

            if let Some(mo) = multiple_of
                && let Some(mo) = integer_multiple_of_divisor(*mo)
            {
                val = (val / mo) * mo;
                if val < low {
                    val += mo;
                }
                if val > high {
                    val -= mo;
                }
            }

            Value::Number(val.into())
        }

        Boolean { enumeration } => {
            if let Some(e) = enumeration
                && !e.is_empty()
            {
                let idx = rng.random_range(0..e.len());
                return e[idx].clone();
            }
            Value::Bool(rng.random_bool(0.5))
        }

        Null { enumeration } => {
            if let Some(e) = enumeration
                && !e.is_empty()
            {
                let idx = rng.random_range(0..e.len());
                return e[idx].clone();
            }
            Value::Null
        }

        Object {
            properties,
            pattern_properties,
            required,
            additional,
            property_names,
            min_properties,
            max_properties,
            enumeration,
            ..
        } => {
            if let Some(e) = enumeration
                && !e.is_empty()
            {
                let idx = rng.random_range(0..e.len());
                return e[idx].clone();
            }

            let min_p: usize = min_properties.unwrap_or(0);
            if min_p == 0
                && properties.is_empty()
                && matches!(
                    additional.kind(),
                    ResolvedNodeKind::Any | ResolvedNodeKind::BoolSchema(true)
                )
                && matches!(
                    property_names.kind(),
                    ResolvedNodeKind::Any | ResolvedNodeKind::BoolSchema(true)
                )
            {
                return Value::Object(Map::new());
            }

            let max_p: usize = max_properties.unwrap_or(usize::MAX);

            for _ in 0..32 {
                let mut map = Map::new();
                for (k, prop_schema) in properties {
                    if !property_name_allows(property_names, k, generator) {
                        continue;
                    }
                    let must_include = required.contains(k);
                    // Optional fields are only included with some probability unless the
                    // schema is unconstrained (`true`/`Any`).  Skipping these avoids
                    // descending into deep recursive refs when it is unnecessary.
                    let include = if must_include {
                        true
                    } else {
                        !matches!(prop_schema.kind(), BoolSchema(true) | Any)
                            && rng.random_bool(0.7)
                    };
                    if include {
                        let val = generator.generate_property_value(
                            k,
                            prop_schema,
                            pattern_properties,
                            rng,
                            depth.saturating_sub(1),
                        );
                        map.insert(k.clone(), val);
                    }
                }

                if !matches!(additional.kind(), BoolSchema(false)) {
                    // If we need to hit `minProperties`, keep inventing additional keys
                    // until we reach the minimum before attempting the probabilistic
                    // extras.
                    let mut attempts = 0;
                    while map.len() < min_p && attempts < 32 {
                        let Some(key) =
                            generate_property_key(property_names, rng, depth, generator)
                        else {
                            break;
                        };
                        if map.contains_key(&key) {
                            attempts += 1;
                            continue;
                        }
                        if properties.contains_key(&key) {
                            attempts += 1;
                            continue;
                        }
                        let val = generator.generate_additional_property_value(
                            &key,
                            pattern_properties,
                            additional,
                            rng,
                            depth.saturating_sub(1),
                        );
                        map.insert(key, val);
                        attempts += 1;
                    }

                    if rng.random_bool(0.3) {
                        let mut attempts = 0;
                        while rng.random_bool(0.5) && (map.len() < max_p) && attempts < 5 {
                            let Some(key) =
                                generate_property_key(property_names, rng, depth, generator)
                            else {
                                break;
                            };
                            if map.contains_key(&key) {
                                attempts += 1;
                                continue;
                            }
                            if properties.contains_key(&key) {
                                attempts += 1;
                                continue;
                            }
                            let val = generator.generate_additional_property_value(
                                &key,
                                pattern_properties,
                                additional,
                                rng,
                                depth.saturating_sub(1),
                            );
                            map.insert(key, val);
                            attempts += 1;
                        }
                    }
                }

                if map.len() < min_p {
                    for (k, prop_schema) in properties {
                        if !property_name_allows(property_names, k, generator) {
                            continue;
                        }
                        if map.contains_key(k) {
                            continue;
                        }
                        let val = generator.generate_property_value(
                            k,
                            prop_schema,
                            pattern_properties,
                            rng,
                            depth.saturating_sub(1),
                        );
                        map.insert(k.clone(), val);
                        if map.len() >= min_p {
                            break;
                        }
                    }
                }

                if map.is_empty()
                    && min_p > 0
                    && !properties.is_empty()
                    && let Some((k, schema)) = properties
                        .iter()
                        .find(|(name, _)| property_name_allows(property_names, name, generator))
                        .or_else(|| properties.iter().next())
                {
                    let val = generator.generate_property_value(
                        k,
                        schema,
                        pattern_properties,
                        rng,
                        depth.saturating_sub(1),
                    );
                    map.insert(k.clone(), val);
                }

                let candidate = Value::Object(map);
                if generator.schema_accepts_value(schema, &candidate) {
                    return candidate;
                }
            }

            random_any(rng, depth)
        }

        Array {
            prefix_items,
            items,
            min_items,
            max_items,
            contains,
            unique_items,
            enumeration,
        } => {
            if let Some(e) = enumeration
                && !e.is_empty()
            {
                let idx = rng.random_range(0..e.len());
                return e[idx].clone();
            }

            let min_contains = contains
                .as_ref()
                .map_or(0, |contains| contains.min_contains);
            let base_min = min_items.unwrap_or(0).max(min_contains);
            let mut max_i = max_items.unwrap_or(base_min.saturating_add(5));
            if matches!(items.kind(), BoolSchema(false)) {
                max_i = max_i.min(prefix_items.len() as u64);
            }
            if max_i < base_min {
                return random_any(rng, depth);
            }
            if base_min == 0
                && prefix_items.is_empty()
                && contains.is_none()
                && !unique_items
                && matches!(items.kind(), BoolSchema(true) | Any)
            {
                return Value::Array(Vec::new());
            }

            if base_min == 0
                && contains
                    .as_ref()
                    .is_some_and(|contains| contains.max_contains == Some(0))
            {
                let empty = Value::Array(Vec::new());
                if generator.schema_accepts_value(schema, &empty) {
                    return empty;
                }
            }

            for _ in 0..32 {
                let length = rng.random_range(base_min..=max_i.min(base_min.saturating_add(5)));
                let candidate = generate_array_candidate(
                    ArrayGenerationSchema {
                        prefix_items,
                        items,
                        contains: contains.as_ref(),
                        unique_items: *unique_items,
                    },
                    length,
                    rng,
                    depth,
                    generator,
                );

                if generator.schema_accepts_value(schema, &candidate) {
                    return candidate;
                }
            }

            random_any(rng, depth)
        }

        IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } => {
            for _ in 0..32 {
                let candidate = if rng.random_bool(0.5)
                    && let Some(t) = then_schema
                {
                    generator.generate_candidate(t, rng, depth.saturating_sub(1))
                } else if let Some(e) = else_schema {
                    generator.generate_candidate(e, rng, depth.saturating_sub(1))
                } else {
                    generator.generate_candidate(if_schema, rng, depth.saturating_sub(1))
                };

                if generator.schema_accepts_value(schema, &candidate) {
                    return candidate;
                }
            }

            random_any(rng, depth)
        }
        Const(v) => v.clone(),
        AllOf(_) => Value::Null,
        AnyOf(_) => Value::Null,
        OneOf(_) => Value::Null,
        Enum(_) => Value::Null,
        _ => random_any(rng, depth),
    }
}

fn integer_multiple_of_divisor(multiple_of: IntegerMultipleOf) -> Option<i64> {
    multiple_of
        .integer_divisor()
        .and_then(|divisor| i64::try_from(divisor).ok())
}

fn object_schema_branch(schema: &ResolvedNode) -> Option<ResolvedNode> {
    use ResolvedNodeKind::*;

    match schema.kind() {
        Object { .. } => Some(schema.clone()),
        AnyOf(subs) | OneOf(subs) => subs.iter().find_map(object_schema_branch),
        IfThenElse {
            then_schema,
            else_schema,
            ..
        } => else_schema
            .as_ref()
            .and_then(object_schema_branch)
            .or_else(|| then_schema.as_ref().and_then(object_schema_branch)),
        _ => None,
    }
}

fn generate_not_value(
    schema: &ResolvedNode,
    negated: &ResolvedNode,
    rng: &mut impl Rng,
    depth: u8,
    generator: &mut ValueGenerator,
) -> Value {
    if let Some(candidate) = negated_schema_counterexample(negated, rng, depth, generator)
        && generator.schema_accepts_value(schema, &candidate)
    {
        return candidate;
    }

    for candidate in fixed_not_candidates() {
        if generator.schema_accepts_value(schema, &candidate) {
            return candidate;
        }
    }

    let forbidden = generator.generate_candidate(negated, rng, depth.saturating_sub(1));
    let candidate = value_type_mismatch(&forbidden);
    if generator.schema_accepts_value(schema, &candidate) {
        return candidate;
    }

    random_any(rng, depth)
}

fn negated_schema_counterexample(
    negated: &ResolvedNode,
    rng: &mut impl Rng,
    depth: u8,
    generator: &mut ValueGenerator,
) -> Option<Value> {
    use ResolvedNodeKind::*;

    Some(match negated.kind() {
        BoolSchema(false) => random_any(rng, depth),
        BoolSchema(true) | Any => return None,
        String { .. } => Value::Number(0.into()),
        Number { .. } | Integer { .. } => Value::String(std::string::String::new()),
        Boolean { .. } => Value::Null,
        Null { .. } => Value::Bool(false),
        Object { .. } => Value::Array(Vec::new()),
        Array { .. } => Value::Object(Map::new()),
        Const(value) => value_type_mismatch(value),
        Enum(values) => values
            .first()
            .map(value_type_mismatch)
            .unwrap_or_else(|| random_any(rng, depth)),
        AllOf(children) => children
            .first()
            .and_then(|child| negated_schema_counterexample(child, rng, depth, generator))
            .unwrap_or_else(|| random_any(rng, depth)),
        AnyOf(_) | OneOf(_) | Not(_) | IfThenElse { .. } => {
            generator.generate_candidate(negated, rng, depth.saturating_sub(1))
        }
        _ => random_any(rng, depth),
    })
}

fn fixed_not_candidates() -> [Value; 8] {
    [
        Value::Null,
        Value::Bool(false),
        Value::Bool(true),
        Value::Number(0.into()),
        Value::String(String::new()),
        Value::Array(Vec::new()),
        Value::Object(Map::new()),
        Value::Object(Map::from_iter([(
            "bar".to_owned(),
            Value::Number(1.into()),
        )])),
    ]
}

fn value_type_mismatch(value: &Value) -> Value {
    match value {
        Value::Null => Value::Bool(false),
        Value::Bool(_) => Value::Null,
        Value::Number(_) => Value::String(String::new()),
        Value::String(_) => Value::Number(0.into()),
        Value::Array(_) => Value::Object(Map::new()),
        Value::Object(_) => Value::Array(Vec::new()),
    }
}

/// Generate a *random JSON Schema* (subset) for fuzzing the value‑generator
/// itself.  The result is raw JSON so it can immediately be passed into the
/// authoritative validator for cross‑checking.
pub fn random_schema(rng: &mut impl Rng, depth: u8) -> Value {
    if depth == 0 {
        return Value::Bool(true);
    }
    match rng.random_range(0..=4) {
        // Primitive types --------------------------------------------------
        0 => {
            // strings with optional minLength / maxLength
            let mut obj = Map::new();
            obj.insert("type".into(), Value::String("string".into()));
            if rng.random_bool(0.5) {
                obj.insert("minLength".into(), rng.random_range(0..5u64).into());
            }
            if rng.random_bool(0.5) {
                obj.insert("maxLength".into(), rng.random_range(5..10u64).into());
            }
            Value::Object(obj)
        }
        1 => {
            // integer range
            let mut obj = Map::new();
            obj.insert("type".into(), Value::String("integer".into()));
            let min = rng.random_range(-20..20);
            let max = min + rng.random_range(0..20);
            obj.insert("minimum".into(), min.into());
            obj.insert("maximum".into(), max.into());
            Value::Object(obj)
        }
        // Array -----------------------------------------------------------
        2 => {
            let mut obj = Map::new();
            obj.insert("type".into(), Value::String("array".into()));
            obj.insert("items".into(), random_schema(rng, depth - 1));
            Value::Object(obj)
        }
        // Object ----------------------------------------------------------
        3 => {
            let mut props = Map::new();
            props.insert("a".into(), random_schema(rng, depth - 1));
            props.insert("b".into(), random_schema(rng, depth - 1));
            let mut obj = Map::new();
            obj.insert("type".into(), Value::String("object".into()));
            obj.insert("properties".into(), Value::Object(props));
            if rng.random_bool(0.5) {
                obj.insert(
                    "required".into(),
                    Value::Array(vec![Value::String("a".into())]),
                );
            }
            Value::Object(obj)
        }
        // Boolean schema --------------------------------------------------
        _ => Value::Bool(true),
    }
}

/// A minimal fallback to produce a random JSON value of any type.
fn random_any(_rng: &mut impl Rng, _depth: u8) -> Value {
    // Always return an empty object – this is valid under the Draft 2020‑12
    // meta‑schema as well as under any unconstrained (`true`) schema.
    Value::Object(Map::new())
}

fn property_name_allows(
    property_names: &ResolvedNode,
    candidate: &str,
    generator: &mut ValueGenerator,
) -> bool {
    generator.schema_accepts_value(property_names, &Value::String(candidate.to_owned()))
}

fn property_name_matches_pattern(pattern: &str, property_name: &str) -> bool {
    Regex::new(pattern)
        .ok()
        .and_then(|regex| regex.is_match(property_name).ok())
        .unwrap_or(false)
}

fn property_matches_any_pattern(
    pattern_properties: &HashMap<String, ResolvedNode>,
    property_name: &str,
) -> bool {
    pattern_properties
        .keys()
        .any(|pattern| property_name_matches_pattern(pattern, property_name))
}

fn matching_pattern_property_schemas<'a>(
    pattern_properties: &'a HashMap<String, ResolvedNode>,
    property_name: &str,
) -> Vec<&'a ResolvedNode> {
    pattern_properties
        .iter()
        .filter_map(|(pattern, schema)| {
            property_name_matches_pattern(pattern, property_name).then_some(schema)
        })
        .collect()
}

fn pattern_property_schemas_accept_value(
    generator: &mut ValueGenerator,
    pattern_properties: &HashMap<String, ResolvedNode>,
    property_name: &str,
    property_value: &Value,
) -> bool {
    matching_pattern_property_schemas(pattern_properties, property_name)
        .into_iter()
        .all(|schema| generator.schema_accepts_value(schema, property_value))
}

fn generate_property_key(
    property_names: &ResolvedNode,
    rng: &mut impl Rng,
    depth: u8,
    generator: &mut ValueGenerator,
) -> Option<String> {
    if matches!(
        property_names.kind(),
        ResolvedNodeKind::Any | ResolvedNodeKind::BoolSchema(true)
    ) {
        return Some(random_key(rng));
    }

    for _ in 0..32 {
        let candidate = match property_names.kind() {
            ResolvedNodeKind::String { .. } => {
                match generator.generate_candidate(property_names, rng, depth.saturating_sub(1)) {
                    Value::String(s) => s,
                    _ => random_key(rng),
                }
            }
            ResolvedNodeKind::Enum(values) => values
                .iter()
                .find_map(|v| v.as_str().map(|s| s.to_owned()))
                .unwrap_or_else(|| random_key(rng)),
            _ => random_key(rng),
        };

        if property_name_allows(property_names, &candidate, generator) {
            return Some(candidate);
        }
    }

    None
}

fn generate_array_candidate(
    schema: ArrayGenerationSchema<'_>,
    length: u64,
    rng: &mut impl Rng,
    depth: u8,
    generator: &mut ValueGenerator,
) -> Value {
    let Some(length) = usize::try_from(length).ok() else {
        return Value::Array(Vec::new());
    };
    let mut values = Vec::with_capacity(length);
    let mut matching_items = 0_u64;

    for index in 0..length {
        let item_schema = schema.prefix_items.get(index).unwrap_or(schema.items);
        let mut value = generate_array_item(
            item_schema,
            schema.contains,
            matching_items,
            length - index,
            rng,
            depth,
            generator,
        );

        if schema.unique_items && values.iter().any(|existing| existing == &value) {
            for _ in 0..32 {
                let candidate = generate_array_item(
                    item_schema,
                    schema.contains,
                    matching_items,
                    length - index,
                    rng,
                    depth,
                    generator,
                );
                if values.iter().all(|existing| existing != &candidate) {
                    value = candidate;
                    break;
                }
            }

            if values.iter().any(|existing| existing == &value)
                && matches!(
                    item_schema.kind(),
                    ResolvedNodeKind::Any | ResolvedNodeKind::BoolSchema(true)
                )
            {
                value = unique_fallback_value(index);
            }
        }

        if let Some(contains) = schema.contains
            && generator.schema_accepts_value(&contains.schema, &value)
        {
            matching_items += 1;
        }
        values.push(value);
    }

    Value::Array(values)
}

fn generate_array_item(
    item_schema: &ResolvedNode,
    contains: Option<&ArrayContains<ResolvedNode>>,
    matching_items: u64,
    remaining_slots: usize,
    rng: &mut impl Rng,
    depth: u8,
    generator: &mut ValueGenerator,
) -> Value {
    match contains {
        None => generator.generate_candidate(item_schema, rng, depth.saturating_sub(1)),
        Some(contains) => {
            let remaining_required = contains.min_contains.saturating_sub(matching_items);
            let must_match = remaining_required >= remaining_slots as u64;
            let can_match = contains
                .max_contains
                .is_none_or(|maximum| matching_items < maximum);

            if must_match {
                generate_array_item_matching_contains(
                    item_schema,
                    &contains.schema,
                    rng,
                    depth,
                    generator,
                )
            } else if !can_match {
                generate_array_item_avoiding_contains(
                    item_schema,
                    &contains.schema,
                    rng,
                    depth,
                    generator,
                )
            } else {
                generator.generate_candidate(item_schema, rng, depth.saturating_sub(1))
            }
        }
    }
}

fn generate_array_item_matching_contains(
    item_schema: &ResolvedNode,
    contains_schema: &ResolvedNode,
    rng: &mut impl Rng,
    depth: u8,
    generator: &mut ValueGenerator,
) -> Value {
    for _ in 0..32 {
        let candidate_schema = if rng.random_bool(0.5) {
            contains_schema
        } else {
            item_schema
        };
        let candidate =
            generator.generate_candidate(candidate_schema, rng, depth.saturating_sub(1));
        let matches_item = generator.schema_accepts_value(item_schema, &candidate);
        let matches_contains = generator.schema_accepts_value(contains_schema, &candidate);
        if matches_item && matches_contains {
            return candidate;
        }
    }

    generator.generate_candidate(item_schema, rng, depth.saturating_sub(1))
}

fn generate_array_item_avoiding_contains(
    item_schema: &ResolvedNode,
    contains_schema: &ResolvedNode,
    rng: &mut impl Rng,
    depth: u8,
    generator: &mut ValueGenerator,
) -> Value {
    for _ in 0..32 {
        let candidate = generator.generate_candidate(item_schema, rng, depth.saturating_sub(1));
        let matches_item = generator.schema_accepts_value(item_schema, &candidate);
        let matches_contains = generator.schema_accepts_value(contains_schema, &candidate);
        if matches_item && !matches_contains {
            return candidate;
        }
    }

    generator.generate_candidate(item_schema, rng, depth.saturating_sub(1))
}

fn unique_fallback_value(index: usize) -> Value {
    let mut value = Map::new();
    value.insert(
        "__jsoncompat_unique".to_owned(),
        Value::Number(index.into()),
    );
    Value::Object(value)
}

fn random_key(rng: &mut impl Rng) -> String {
    random_string(rng, 3..8)
}

fn random_string(rng: &mut impl Rng, len_range: std::ops::Range<usize>) -> String {
    let len = rng.random_range(len_range);
    (0..len)
        .map(|_| rng.sample(rand::distr::Alphanumeric) as char)
        .collect()
}

fn literal_from_simple_pattern(pattern: &str) -> Option<String> {
    let body = pattern.strip_prefix('^')?.strip_suffix('$')?;
    if body.is_empty() {
        return None;
    }

    let mut literal = String::new();
    let mut chars = body.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            if ch.is_ascii_alphanumeric() || ch == ' ' || ch == '-' || ch == '_' {
                literal.push(ch);
                continue;
            }
            return None;
        }

        let escaped = match chars.next()? {
            't' => '\t',
            'n' => '\n',
            'r' => '\r',
            '\\' => '\\',
            'd' => '0',
            'D' => 'a',
            's' => ' ',
            'S' => 'a',
            'w' => 'a',
            'W' => '!',
            'c' => {
                let control = chars.next()?;
                if control.is_ascii_alphabetic() {
                    ((control.to_ascii_uppercase() as u8) & 0x1f) as char
                } else {
                    return None;
                }
            }
            other if other.is_ascii_punctuation() => other,
            _ => return None,
        };
        literal.push(escaped);
    }

    Some(literal)
}

// lots to improve here:
// - anyof generation
// - better random any
// - better not handling
// - lots of other stuff

#[cfg(test)]
mod tests {
    use super::*;
    use json_schema_ast::ResolvedSchema;
    use rand::{SeedableRng, rngs::StdRng};
    use serde_json::json;

    fn resolve(raw: Value) -> ResolvedSchema {
        ResolvedSchema::from_json(&raw).unwrap()
    }

    #[test]
    fn object_required_properties_not_empty() {
        let schema = build_required_object_schema();
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..50 {
            let value = generate_value(&schema, &mut rng, 5).unwrap();
            let obj = value.as_object().expect("expected object");
            assert!(obj.contains_key("class"));
            assert!(
                schema.is_valid(&value).unwrap(),
                "generated invalid value: {value}"
            );
        }
    }

    #[test]
    fn recursive_allof_with_cyclic_object_branch_does_not_stack_overflow() {
        let schema = resolve(json!({
            "$defs": {
                "A": {
                    "allOf": [
                        {
                            "type": "object",
                            "properties": {
                                "next": { "$ref": "#/$defs/A" }
                            }
                        },
                        { "type": "number" }
                    ]
                }
            },
            "$ref": "#/$defs/A"
        }));

        let mut rng = StdRng::seed_from_u64(42);
        assert!(matches!(
            generate_value(&schema, &mut rng, 5),
            Err(GenerateError::ExhaustedAttempts { .. })
        ));
    }

    #[test]
    fn recursive_anyof_branch_does_not_get_serialized_before_generation() {
        let schema = resolve(json!({
            "$defs": {
                "Node": {
                    "properties": {
                        "next": { "$ref": "#/$defs/Node" }
                    }
                }
            },
            "anyOf": [
                { "$ref": "#/$defs/Node" },
                { "type": "string" }
            ]
        }));

        let mut rng = StdRng::seed_from_u64(7);
        let _ = generate_value(&schema, &mut rng, 5).unwrap();
    }

    #[test]
    fn recursive_contains_with_zero_max_contains_only_generates_empty_arrays() {
        let schema = resolve(json!({
            "$defs": {
                "A": {
                    "type": "array",
                    "items": { "$ref": "#/$defs/A" },
                    "contains": { "$ref": "#/$defs/A" },
                    "minContains": 0,
                    "maxContains": 0
                }
            },
            "$ref": "#/$defs/A"
        }));

        let mut rng = StdRng::seed_from_u64(99);
        for _ in 0..50 {
            let value = generate_value(&schema, &mut rng, 8).unwrap();
            assert_eq!(value, json!([]), "generated invalid value: {value}");
        }
    }

    #[test]
    fn allof_object_generation_keeps_extra_keys_for_min_properties() {
        let schema = resolve(json!({
            "allOf": [
                {
                    "type": "object",
                    "minProperties": 1
                },
                {
                    "type": "object"
                }
            ]
        }));

        let mut rng = StdRng::seed_from_u64(12);
        for _ in 0..20 {
            let value = generate_value(&schema, &mut rng, 5).unwrap();
            assert!(
                schema.is_valid(&value).unwrap(),
                "generated invalid value: {value}"
            );
        }
    }

    fn build_required_object_schema() -> ResolvedSchema {
        let raw = json!({
            "type": "object",
            "properties": {
                "class": {"const": "MyClass"},
                "opt": {"type": "integer"}
            },
            "required": ["class"]
        });
        resolve(raw)
    }
}

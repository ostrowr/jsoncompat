mod format_gen;
mod regex_gen;

use json_schema_ast::{ArrayContains, JSONSchema, SchemaNode, SchemaNodeKind, compile};
use rand::{Rng, RngExt};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use json_schema_ast::SchemaNodeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RecursiveValidationFrame {
    schema_id: SchemaNodeId,
    value_address: usize,
}

/// Stateful value generator that caches compiled subvalidators by AST node identity.
#[derive(Default)]
pub struct ValueGenerator {
    validators: HashMap<SchemaNodeId, Option<Rc<JSONSchema>>>,
}

#[derive(Clone, Copy)]
struct ArrayGenerationSchema<'a> {
    prefix_items: &'a [SchemaNode],
    items: &'a SchemaNode,
    contains: Option<&'a ArrayContains<SchemaNode>>,
    unique_items: bool,
}

impl ValueGenerator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate a random JSON value *intended* to satisfy `schema`.
    /// We limit recursion with `depth`.
    pub fn generate_value(&mut self, schema: &SchemaNode, rng: &mut impl Rng, depth: u8) -> Value {
        generate_value_with_context(schema, rng, depth, self)
    }

    fn compile_schema_node(&mut self, schema: &SchemaNode) -> Option<Rc<JSONSchema>> {
        let node_id = schema.id();
        if let Some(validator) = self.validators.get(&node_id) {
            return validator.clone();
        }

        let validator = if schema_contains_cycle(schema, &mut Vec::new()) {
            None
        } else {
            compile(&schema.to_json()).ok().map(Rc::new)
        };
        self.validators.insert(node_id, validator.clone());
        validator
    }

    fn schema_accepts_value(&mut self, schema: &SchemaNode, value: &Value) -> bool {
        self.schema_accepts_value_inner(schema, value, &mut HashSet::new())
    }

    fn schema_accepts_value_inner(
        &mut self,
        schema: &SchemaNode,
        value: &Value,
        active: &mut HashSet<RecursiveValidationFrame>,
    ) -> bool {
        if let Some(validator) = self.compile_schema_node(schema) {
            return validator.is_valid(value);
        }

        let frame = RecursiveValidationFrame {
            schema_id: schema.id(),
            value_address: std::ptr::from_ref(value) as usize,
        };
        if !active.insert(frame) {
            // Re-entering the same schema on the same JSON value does not make
            // progress (`A = anyOf(string, A)` on `[]`). Fail closed here while
            // allowing productive recursion over child values with distinct addresses.
            return false;
        }

        let is_valid = match schema.kind() {
            SchemaNodeKind::BoolSchema(valid) => *valid,
            SchemaNodeKind::Any => true,
            SchemaNodeKind::String {
                min_length,
                max_length,
                enumeration,
                ..
            } => value.as_str().is_some_and(|string_value| {
                string_length_in_range(string_value, *min_length, *max_length)
                    && enum_contains_value(
                        enumeration.as_deref(),
                        &Value::String(string_value.to_owned()),
                    )
            }),
            SchemaNodeKind::Number {
                minimum,
                maximum,
                exclusive_minimum,
                exclusive_maximum,
                multiple_of,
                enumeration,
            } => value.as_f64().is_some_and(|number_value| {
                numeric_value_in_range(
                    number_value,
                    *minimum,
                    *exclusive_minimum,
                    *maximum,
                    *exclusive_maximum,
                ) && value_is_multiple_of(number_value, *multiple_of)
                    && enum_contains_numeric_value(enumeration.as_deref(), number_value)
            }),
            SchemaNodeKind::Integer {
                minimum,
                maximum,
                exclusive_minimum,
                exclusive_maximum,
                multiple_of,
                enumeration,
            } => value.as_f64().is_some_and(|number_value| {
                number_value.fract() == 0.0
                    && numeric_value_in_range(
                        number_value,
                        minimum.map(|bound| bound as f64),
                        *exclusive_minimum,
                        maximum.map(|bound| bound as f64),
                        *exclusive_maximum,
                    )
                    && value_is_multiple_of(number_value, *multiple_of)
                    && enum_contains_numeric_value(enumeration.as_deref(), number_value)
            }),
            SchemaNodeKind::Boolean { enumeration } => value
                .as_bool()
                .is_some_and(|_| enum_contains_value(enumeration.as_deref(), value)),
            SchemaNodeKind::Null { enumeration } => {
                value.is_null() && enum_contains_value(enumeration.as_deref(), value)
            }
            SchemaNodeKind::Object {
                properties,
                required,
                additional,
                property_names,
                min_properties,
                max_properties,
                dependent_required,
                enumeration,
            } => value.as_object().is_some_and(|object_value| {
                enum_contains_value(enumeration.as_deref(), value)
                    && min_properties.is_none_or(|minimum| object_value.len() >= minimum)
                    && max_properties.is_none_or(|maximum| object_value.len() <= maximum)
                    && required.iter().all(|name| object_value.contains_key(name))
                    && dependent_required.iter().all(|(trigger, dependencies)| {
                        !object_value.contains_key(trigger)
                            || dependencies
                                .iter()
                                .all(|dependency| object_value.contains_key(dependency))
                    })
                    && object_value.iter().all(|(property_name, property_value)| {
                        let property_name_value = Value::String(property_name.clone());
                        self.schema_accepts_value_inner(
                            property_names,
                            &property_name_value,
                            active,
                        ) && self.schema_accepts_value_inner(
                            properties.get(property_name).unwrap_or(additional),
                            property_value,
                            active,
                        )
                    })
            }),
            SchemaNodeKind::Array {
                prefix_items,
                items,
                min_items,
                max_items,
                contains,
                unique_items,
                enumeration,
            } => value.as_array().is_some_and(|array_value| {
                enum_contains_value(enumeration.as_deref(), value)
                    && min_items.is_none_or(|minimum| array_value.len() as u64 >= minimum)
                    && max_items.is_none_or(|maximum| array_value.len() as u64 <= maximum)
                    && (!unique_items || array_values_are_unique(array_value))
                    && array_value.iter().enumerate().all(|(index, item)| {
                        let item_schema = prefix_items.get(index).unwrap_or(items);
                        self.schema_accepts_value_inner(item_schema, item, active)
                    })
                    && contains.as_ref().is_none_or(|contains| {
                        let matching_items = array_value
                            .iter()
                            .filter(|item| {
                                self.schema_accepts_value_inner(&contains.schema, item, active)
                            })
                            .count() as u64;
                        matching_items >= contains.min_contains
                            && contains
                                .max_contains
                                .is_none_or(|maximum| matching_items <= maximum)
                    })
            }),
            SchemaNodeKind::Defs(_) => true,
            SchemaNodeKind::AllOf(children) => children
                .iter()
                .all(|child| self.schema_accepts_value_inner(child, value, active)),
            SchemaNodeKind::AnyOf(children) => children
                .iter()
                .any(|child| self.schema_accepts_value_inner(child, value, active)),
            SchemaNodeKind::OneOf(children) => {
                children
                    .iter()
                    .filter(|child| self.schema_accepts_value_inner(child, value, active))
                    .count()
                    == 1
            }
            SchemaNodeKind::Not(child) => !self.schema_accepts_value_inner(child, value, active),
            SchemaNodeKind::IfThenElse {
                if_schema,
                then_schema,
                else_schema,
            } => {
                if self.schema_accepts_value_inner(if_schema, value, active) {
                    then_schema.as_ref().is_none_or(|then_schema| {
                        self.schema_accepts_value_inner(then_schema, value, active)
                    })
                } else {
                    else_schema.as_ref().is_none_or(|else_schema| {
                        self.schema_accepts_value_inner(else_schema, value, active)
                    })
                }
            }
            SchemaNodeKind::Const(expected) => value == expected,
            SchemaNodeKind::Enum(values) => values.contains(value),
            SchemaNodeKind::Type(_)
            | SchemaNodeKind::Minimum(_)
            | SchemaNodeKind::Maximum(_)
            | SchemaNodeKind::Required(_)
            | SchemaNodeKind::AdditionalProperties(_)
            | SchemaNodeKind::Format(_)
            | SchemaNodeKind::ContentEncoding(_)
            | SchemaNodeKind::ContentMediaType(_)
            | SchemaNodeKind::Title(_)
            | SchemaNodeKind::Description(_)
            | SchemaNodeKind::Default(_)
            | SchemaNodeKind::Examples(_)
            | SchemaNodeKind::ReadOnly(_)
            | SchemaNodeKind::WriteOnly(_)
            | SchemaNodeKind::Ref(_)
            | _ => false,
        };

        active.remove(&frame);
        is_valid
    }
}

/// Generate a random JSON value *intended* to satisfy `schema`.
/// We limit recursion with `depth`.
pub fn generate_value(schema: &SchemaNode, rng: &mut impl Rng, depth: u8) -> Value {
    ValueGenerator::new().generate_value(schema, rng, depth)
}

fn generate_value_with_context(
    schema: &SchemaNode,
    rng: &mut impl Rng,
    depth: u8,
    generator: &mut ValueGenerator,
) -> Value {
    if depth == 0 {
        return Value::Null;
    }

    use SchemaNodeKind::*;

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
                            required,
                            additional,
                            min_properties,
                            ..
                        } = sub.kind()
                            && let Value::Object(obj) =
                                generator.generate_value(sub, rng, depth.saturating_sub(1))
                        {
                            for (k, v) in obj {
                                if required.contains(&k)
                                    || !matches!(additional.kind(), BoolSchema(true) | Any)
                                    || properties.contains_key(&k)
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
                    let mut missing: HashMap<std::string::String, Option<SchemaNode>> =
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
                                generator.generate_value(&schema, rng, depth.saturating_sub(1))
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
                    generator.generate_value(non_trivial[index], rng, depth.saturating_sub(1))
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
                    generator.generate_value(&subs[index], rng, depth.saturating_sub(1));
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
                    generator.generate_value(&candidate_schema, rng, depth.saturating_sub(1));

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

        Not(_) => random_any(rng, depth),

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

            if let Some(mo_f) = multiple_of
                && *mo_f > 0.0
            {
                let mo = (*mo_f).round() as i64;
                if mo != 0 {
                    val = (val / mo) * mo;
                    if val < low {
                        val += mo;
                    }
                    if val > high {
                        val -= mo;
                    }
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
                    SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true)
                )
                && matches!(
                    property_names.kind(),
                    SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true)
                )
            {
                return Value::Object(Map::new());
            }

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
                    !matches!(prop_schema.kind(), BoolSchema(true) | Any) && rng.random_bool(0.7)
                };
                if include {
                    let val = generator.generate_value(prop_schema, rng, depth.saturating_sub(1));
                    map.insert(k.clone(), val);
                }
            }

            let max_p: usize = max_properties.unwrap_or(usize::MAX);

            if !matches!(additional.kind(), BoolSchema(false)) {
                // If we need to hit `minProperties`, keep inventing additional keys
                // until we reach the minimum before attempting the probabilistic
                // extras.
                while map.len() < min_p {
                    let Some(key) = generate_property_key(property_names, rng, depth, generator)
                    else {
                        break;
                    };
                    if map.contains_key(&key) {
                        continue;
                    }
                    let val = generator.generate_value(additional, rng, depth.saturating_sub(1));
                    map.insert(key, val);
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
                        let val =
                            generator.generate_value(additional, rng, depth.saturating_sub(1));
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
                    let val = generator.generate_value(prop_schema, rng, depth.saturating_sub(1));
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
                let val = generator.generate_value(schema, rng, depth.saturating_sub(1));
                map.insert(k.clone(), val);
            }

            Value::Object(map)
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

        Ref(_) => random_any(rng, depth),
        Defs(_) => Value::Null,
        IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } => {
            for _ in 0..32 {
                let candidate = if rng.random_bool(0.5)
                    && let Some(t) = then_schema
                {
                    generator.generate_value(t, rng, depth.saturating_sub(1))
                } else if let Some(e) = else_schema {
                    generator.generate_value(e, rng, depth.saturating_sub(1))
                } else {
                    generator.generate_value(if_schema, rng, depth.saturating_sub(1))
                };

                if generator.schema_accepts_value(schema, &candidate) {
                    return candidate;
                }
            }

            random_any(rng, depth)
        }
        Const(v) => v.clone(),
        Type(_) => Value::Null,
        Minimum(_) => Value::Null,
        Maximum(_) => Value::Null,
        Required(_) => Value::Null,
        AdditionalProperties(_) => Value::Null,
        Format(_) => Value::Null,
        ContentEncoding(_) => Value::Null,
        ContentMediaType(_) => Value::Null,
        Title(_) => Value::Null,
        Description(_) => Value::Null,
        Default(_) => Value::Null,
        Examples(_) => Value::Null,
        ReadOnly(_) => Value::Null,
        WriteOnly(_) => Value::Null,
        AllOf(_) => Value::Null,
        AnyOf(_) => Value::Null,
        OneOf(_) => Value::Null,
        Enum(_) => Value::Null,
        _ => random_any(rng, depth),
    }
}

fn object_schema_branch(schema: &SchemaNode) -> Option<SchemaNode> {
    use SchemaNodeKind::*;

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

fn schema_contains_cycle(schema: &SchemaNode, path: &mut Vec<SchemaNode>) -> bool {
    if path.iter().any(|ancestor| ancestor.ptr_eq(schema)) {
        return true;
    }

    let children = schema_children(schema);
    if children.is_empty() {
        return false;
    }

    path.push(schema.clone());
    let contains_cycle = children
        .iter()
        .any(|child| schema_contains_cycle(child, path));
    path.pop();
    contains_cycle
}

fn schema_children(schema: &SchemaNode) -> Vec<SchemaNode> {
    use SchemaNodeKind::*;

    match schema.kind() {
        AllOf(children) | AnyOf(children) | OneOf(children) => children.clone(),
        Not(child) | AdditionalProperties(child) => vec![child.clone()],
        IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } => std::iter::once(if_schema.clone())
            .chain(then_schema.iter().cloned())
            .chain(else_schema.iter().cloned())
            .collect(),
        Object {
            properties,
            additional,
            property_names,
            ..
        } => properties
            .values()
            .cloned()
            .chain(std::iter::once(additional.clone()))
            .chain(std::iter::once(property_names.clone()))
            .collect(),
        Array {
            prefix_items,
            items,
            contains,
            ..
        } => prefix_items
            .iter()
            .cloned()
            .chain(std::iter::once(items.clone()))
            .chain(contains.iter().map(|contains| contains.schema.clone()))
            .collect(),
        Defs(map) => map.values().cloned().collect(),
        _ => Vec::new(),
    }
}

fn property_name_allows(
    property_names: &SchemaNode,
    candidate: &str,
    generator: &mut ValueGenerator,
) -> bool {
    generator.schema_accepts_value(property_names, &Value::String(candidate.to_owned()))
}

fn generate_property_key(
    property_names: &SchemaNode,
    rng: &mut impl Rng,
    depth: u8,
    generator: &mut ValueGenerator,
) -> Option<String> {
    if matches!(
        property_names.kind(),
        SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true)
    ) {
        return Some(random_key(rng));
    }

    for _ in 0..32 {
        let candidate = match property_names.kind() {
            SchemaNodeKind::String { .. } => {
                match generator.generate_value(property_names, rng, depth.saturating_sub(1)) {
                    Value::String(s) => s,
                    _ => random_key(rng),
                }
            }
            SchemaNodeKind::Enum(values) => values
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
                    SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true)
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
    item_schema: &SchemaNode,
    contains: Option<&ArrayContains<SchemaNode>>,
    matching_items: u64,
    remaining_slots: usize,
    rng: &mut impl Rng,
    depth: u8,
    generator: &mut ValueGenerator,
) -> Value {
    match contains {
        None => generator.generate_value(item_schema, rng, depth.saturating_sub(1)),
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
                generator.generate_value(item_schema, rng, depth.saturating_sub(1))
            }
        }
    }
}

fn generate_array_item_matching_contains(
    item_schema: &SchemaNode,
    contains_schema: &SchemaNode,
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
        let candidate = generator.generate_value(candidate_schema, rng, depth.saturating_sub(1));
        let matches_item = generator.schema_accepts_value(item_schema, &candidate);
        let matches_contains = generator.schema_accepts_value(contains_schema, &candidate);
        if matches_item && matches_contains {
            return candidate;
        }
    }

    generator.generate_value(item_schema, rng, depth.saturating_sub(1))
}

fn generate_array_item_avoiding_contains(
    item_schema: &SchemaNode,
    contains_schema: &SchemaNode,
    rng: &mut impl Rng,
    depth: u8,
    generator: &mut ValueGenerator,
) -> Value {
    for _ in 0..32 {
        let candidate = generator.generate_value(item_schema, rng, depth.saturating_sub(1));
        let matches_item = generator.schema_accepts_value(item_schema, &candidate);
        let matches_contains = generator.schema_accepts_value(contains_schema, &candidate);
        if matches_item && !matches_contains {
            return candidate;
        }
    }

    generator.generate_value(item_schema, rng, depth.saturating_sub(1))
}

fn unique_fallback_value(index: usize) -> Value {
    let mut value = Map::new();
    value.insert(
        "__jsoncompat_unique".to_owned(),
        Value::Number(index.into()),
    );
    Value::Object(value)
}

fn array_values_are_unique(values: &[Value]) -> bool {
    values
        .iter()
        .enumerate()
        .all(|(index, value)| values[..index].iter().all(|seen| seen != value))
}

fn string_length_in_range(value: &str, minimum: Option<u64>, maximum: Option<u64>) -> bool {
    let length = value.chars().count() as u64;
    minimum.is_none_or(|minimum| length >= minimum)
        && maximum.is_none_or(|maximum| length <= maximum)
}

fn numeric_value_in_range(
    value: f64,
    minimum: Option<f64>,
    exclusive_minimum: bool,
    maximum: Option<f64>,
    exclusive_maximum: bool,
) -> bool {
    let above_minimum = match minimum {
        None => true,
        Some(minimum) if exclusive_minimum => value > minimum,
        Some(minimum) => value >= minimum,
    };
    let below_maximum = match maximum {
        None => true,
        Some(maximum) if exclusive_maximum => value < maximum,
        Some(maximum) => value <= maximum,
    };
    above_minimum && below_maximum
}

fn enum_contains_value(enumeration: Option<&[Value]>, value: &Value) -> bool {
    enumeration.is_none_or(|enumeration| enumeration.contains(value))
}

fn enum_contains_numeric_value(enumeration: Option<&[Value]>, value: f64) -> bool {
    enumeration.is_none_or(|enumeration| {
        enumeration
            .iter()
            .any(|expected| expected.as_f64().is_some_and(|expected| expected == value))
    })
}

fn value_is_multiple_of(value: f64, multiple_of: Option<f64>) -> bool {
    let Some(multiple_of) = multiple_of else {
        return true;
    };
    if multiple_of <= 0.0 {
        return false;
    }
    if let (Some(value), Some(multiple_of)) = (
        exact_positive_integer(value.abs()),
        exact_positive_integer(multiple_of),
    ) {
        return value % multiple_of == 0;
    }

    let ratio = value / multiple_of;
    (ratio - ratio.round()).abs() <= f64::EPSILON * ratio.abs().max(1.0) * 4.0
}

fn exact_positive_integer(value: f64) -> Option<u64> {
    if !value.is_finite() || value <= 0.0 || value.fract() != 0.0 || value > u64::MAX as f64 {
        return None;
    }

    let integer = value as u64;
    ((integer as f64) == value).then_some(integer)
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
    use json_schema_ast::build_and_resolve_schema;
    use rand::{SeedableRng, rngs::StdRng};
    use serde_json::json;

    #[test]
    fn object_required_properties_not_empty() {
        let schema = build_required_object_schema();
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..50 {
            let value = generate_value(&schema, &mut rng, 5);
            let obj = value.as_object().expect("expected object");
            assert!(obj.contains_key("class"));
        }
    }

    #[test]
    fn recursive_allof_with_cyclic_object_branch_does_not_stack_overflow() {
        let schema = build_and_resolve_schema(&json!({
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
        }))
        .unwrap();

        let mut rng = StdRng::seed_from_u64(42);
        let _ = generate_value(&schema, &mut rng, 5);
    }

    #[test]
    fn recursive_anyof_branch_does_not_get_serialized_before_generation() {
        let schema = build_and_resolve_schema(&json!({
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
        }))
        .unwrap();

        let mut rng = StdRng::seed_from_u64(7);
        let _ = generate_value(&schema, &mut rng, 5);
    }

    #[test]
    fn recursive_contains_with_zero_max_contains_only_generates_empty_arrays() {
        let schema = build_and_resolve_schema(&json!({
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
        }))
        .unwrap();

        let mut rng = StdRng::seed_from_u64(99);
        for _ in 0..50 {
            let value = generate_value(&schema, &mut rng, 8);
            assert_eq!(value, json!([]), "generated invalid value: {value}");
        }
    }

    #[test]
    fn allof_object_generation_keeps_extra_keys_for_min_properties() {
        let schema = build_and_resolve_schema(&json!({
            "allOf": [
                {
                    "type": "object",
                    "minProperties": 1
                },
                {
                    "type": "object"
                }
            ]
        }))
        .unwrap();

        let compiled = compile(&schema.to_json()).unwrap();
        let mut rng = StdRng::seed_from_u64(12);
        for _ in 0..20 {
            let value = generate_value(&schema, &mut rng, 5);
            assert!(
                compiled.is_valid(&value),
                "generated invalid value: {value}"
            );
        }
    }

    fn build_required_object_schema() -> SchemaNode {
        let raw = json!({
            "type": "object",
            "properties": {
                "class": {"const": "MyClass"},
                "opt": {"type": "integer"}
            },
            "required": ["class"]
        });
        build_and_resolve_schema(&raw).unwrap()
    }
}

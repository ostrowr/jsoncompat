use json_schema_ast::{SchemaNode, SchemaNodeKind};
use rand::Rng;
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Satisfiability {
    Always,
    Sometimes,
    Never,
}

/// Best-effort static satisfiability check.  The goal is not to be
/// complete, but to quickly classify obviously impossible schemas (e.g.
/// `{"not": true}` or `{"allOf": [true, false]}`) so that the fuzzer can
/// skip them instead of repeatedly generating invalid instances.
pub fn satisfiability(schema: &SchemaNode) -> Satisfiability {
    satisfiability_inner(schema, 16)
}

fn satisfiability_inner(schema: &SchemaNode, depth: u8) -> Satisfiability {
    use Satisfiability::*;
    use SchemaNodeKind::*;

    if depth == 0 {
        return Sometimes;
    }

    match &*schema.borrow() {
        BoolSchema(false) => Never,
        BoolSchema(true) | Any => Always,

        Enum(vals) => {
            if vals.is_empty() {
                Never
            } else {
                Sometimes
            }
        }
        Const(_) => Sometimes,

        AllOf(subs) => {
            if subs.is_empty() {
                return Always;
            }
            let mut saw_maybe = false;
            for s in subs {
                match satisfiability_inner(s, depth - 1) {
                    Never => return Never,
                    Sometimes => saw_maybe = true,
                    Always => {}
                }
            }
            if saw_maybe {
                Sometimes
            } else {
                Always
            }
        }
        AnyOf(subs) => {
            if subs.is_empty() {
                return Never;
            }
            let mut saw_maybe = false;
            for s in subs {
                match satisfiability_inner(s, depth - 1) {
                    Always => return Always,
                    Sometimes => saw_maybe = true,
                    Never => {}
                }
            }
            if saw_maybe {
                Sometimes
            } else {
                Never
            }
        }
        OneOf(subs) => {
            if subs.is_empty() {
                return Never;
            }
            let mut always = 0;
            let mut maybe = 0;
            for s in subs {
                match satisfiability_inner(s, depth - 1) {
                    Always => always += 1,
                    Sometimes => maybe += 1,
                    Never => {}
                }
            }
            if always > 1 || (always == 0 && maybe == 0) {
                Never
            } else if always == 1 && maybe == 0 {
                Always
            } else {
                Sometimes
            }
        }
        Not(sub) => match satisfiability_inner(sub, depth - 1) {
            Always => Never,
            Never => Always,
            Sometimes => Sometimes,
        },
        IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } => match satisfiability_inner(if_schema, depth - 1) {
            Always => then_schema
                .as_ref()
                .map(|s| satisfiability_inner(s, depth - 1))
                .unwrap_or(Always),
            Never => else_schema
                .as_ref()
                .map(|s| satisfiability_inner(s, depth - 1))
                .unwrap_or(Always),
            Sometimes => Sometimes,
        },

        String { enumeration, .. }
        | Number { enumeration, .. }
        | Integer { enumeration, .. }
        | Boolean { enumeration, .. }
        | Null { enumeration, .. } => {
            if let Some(list) = enumeration {
                if list.is_empty() {
                    Never
                } else {
                    Sometimes
                }
            } else {
                Sometimes
            }
        }
        Object { enumeration, .. } => match enumeration {
            Some(list) if list.is_empty() => Never,
            Some(_) => Sometimes,
            None => Sometimes,
        },
        Array {
            enumeration,
            contains,
            min_contains,
            max_contains,
            ..
        } => {
            if let Some(list) = enumeration {
                if list.is_empty() {
                    return Never;
                }
            }
            if let (Some(min_c), Some(max_c)) = (min_contains, max_contains) {
                if min_c > max_c {
                    return Never;
                }
            }
            if min_contains.is_some() && contains.is_none() {
                return Never;
            }
            if let Some(sub) = contains {
                if matches!(satisfiability_inner(sub, depth - 1), Never) {
                    return Never;
                }
            }
            Sometimes
        }

        Defs(_) => Sometimes,

        // These scalar keyword nodes model validation details that don't
        // affect satisfiability on their own.
        Type(_)
        | Minimum(_)
        | Maximum(_)
        | Required(_)
        | AdditionalProperties(_)
        | Format(_)
        | ContentEncoding(_)
        | ContentMediaType(_)
        | Title(_)
        | Description(_)
        | Default(_)
        | Examples(_)
        | ReadOnly(_)
        | WriteOnly(_)
        | Ref(_) => Sometimes,
    }
}

/// Generate a random JSON value *intended* to satisfy `schema`.
/// We limit recursion with `depth`.
pub fn generate_value(schema: &SchemaNode, rng: &mut impl Rng, depth: u8) -> Value {
    if depth == 0 {
        return Value::Null;
    }
    if matches!(satisfiability(schema), Satisfiability::Never) {
        return Value::Null;
    }

    use SchemaNodeKind::*;

    match &*schema.borrow() {
        BoolSchema(false) => Value::Null,
        BoolSchema(true) | Any => random_any(rng, depth),

        Enum(vals) if !vals.is_empty() => {
            let idx = rng.gen_range(0..vals.len());
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
            if subs
                .iter()
                .any(|s| matches!(&*s.borrow(), BoolSchema(false)))
            {
                return Value::Null;
            }

            if subs.iter().all(|s| matches!(&*s.borrow(), Object { .. })) {
                use std::collections::HashMap;

                let mut combined = Map::new();
                for sub in subs {
                    if let Value::Object(obj) = generate_value(sub, rng, depth.saturating_sub(1)) {
                        for (k, v) in obj {
                            combined.insert(k, v);
                        }
                    }
                }

                // Ensure that every branch's required properties exist in the
                // merged object.  The generator is probabilistic, so it is
                // possible that the fast generation above skipped an optional
                // property that later turned out to be required by another
                // branch.  We deterministically fill any such gaps here.
                let mut missing: HashMap<std::string::String, SchemaNode> = HashMap::new();
                for sub in subs {
                    if let Object {
                        properties,
                        required,
                        ..
                    } = &*sub.borrow()
                    {
                        for req in required {
                            if !combined.contains_key(req) {
                                if let Some(prop_schema) = properties.get(req) {
                                    missing.insert(req.clone(), prop_schema.clone());
                                } else {
                                    missing.insert(req.clone(), SchemaNode::any());
                                }
                            }
                        }
                    }
                }

                for (k, schema) in missing {
                    let val = generate_value(&schema, rng, depth.saturating_sub(1));
                    combined.insert(k, val);
                }

                return Value::Object(combined);
            }

            let validators: Vec<_> = subs
                .iter()
                .map(|s| json_schema_ast::compile(&s.to_json()).ok())
                .collect();

            for _ in 0..32 {
                let pick = rng.gen_range(0..subs.len());
                let candidate = generate_value(&subs[pick], rng, depth.saturating_sub(1));
                if validators.iter().all(|v| {
                    v.as_ref()
                        .map(|val| val.is_valid(&candidate))
                        .unwrap_or(true)
                }) {
                    return candidate;
                }
            }

            random_any(rng, depth)
        }

        AnyOf(subs) if !subs.is_empty() => {
            let idx = rng.gen_range(0..subs.len());
            generate_value(&subs[idx], rng, depth.saturating_sub(1))
        }

        OneOf(subs) if !subs.is_empty() => {
            let validators: Vec<_> = subs
                .iter()
                .map(|s| json_schema_ast::compile(&s.to_json()).ok())
                .collect();

            for _ in 0..32 {
                let pick = rng.gen_range(0..subs.len());
                let candidate = generate_value(&subs[pick], rng, depth.saturating_sub(1));

                let mut ok = 0;
                for v in validators.iter().flatten() {
                    if v.is_valid(&candidate) {
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

        Not(sub) => generate_value_not(sub, rng, depth),

        String {
            min_length,
            max_length,
            enumeration,
            ..
        } => {
            if let Some(e) = enumeration {
                if !e.is_empty() {
                    let idx = rng.gen_range(0..e.len());
                    return e[idx].clone();
                }
            }

            let len_min = min_length.unwrap_or(0);
            let len_max = max_length.unwrap_or(len_min + 5).max(len_min);
            let length = if len_min <= len_max {
                rng.gen_range(len_min..=len_max.min(len_min + 10))
            } else {
                len_min
            };
            let s: std::string::String = (0..length)
                .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
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
            if let Some(e) = enumeration {
                if !e.is_empty() {
                    let idx = rng.gen_range(0..e.len());
                    return e[idx].clone();
                }
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
            let mut val = rng.gen_range(low..=high);

            if let Some(mo) = multiple_of {
                if *mo > 0.0 {
                    let k = (val / *mo).floor();
                    val = k * *mo;
                    if val < low || val > high {
                        let k = (low / *mo).ceil();
                        val = k * *mo;
                    }
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
            if let Some(e) = enumeration {
                if !e.is_empty() {
                    let idx = rng.gen_range(0..e.len());
                    return e[idx].clone();
                }
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
            let mut val = rng.gen_range(low..=high);

            if let Some(mo_f) = multiple_of {
                if *mo_f > 0.0 {
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
            }

            Value::Number(val.into())
        }

        Boolean { enumeration, .. } => {
            if let Some(e) = enumeration {
                if !e.is_empty() {
                    let idx = rng.gen_range(0..e.len());
                    return e[idx].clone();
                }
            }
            Value::Bool(rng.gen_bool(0.5))
        }

        Null { enumeration, .. } => {
            if let Some(e) = enumeration {
                if !e.is_empty() {
                    let idx = rng.gen_range(0..e.len());
                    return e[idx].clone();
                }
            }
            Value::Null
        }

        Object {
            properties,
            required,
            additional,
            min_properties,
            max_properties,
            enumeration,
            ..
        } => {
            if let Some(e) = enumeration {
                if !e.is_empty() {
                    let idx = rng.gen_range(0..e.len());
                    return e[idx].clone();
                }
            }

            let mut map = Map::new();
            for (k, prop_schema) in properties {
                let must_include = required.contains(k);
                if matches!(satisfiability(prop_schema), Satisfiability::Never) {
                    if must_include {
                        return Value::Null;
                    }
                    continue;
                }
                // Optional fields are only included with some probability unless the
                // schema is unconstrained (`true`/`Any`).  Skipping these avoids
                // descending into deep recursive refs when it is unnecessary.
                let include = if must_include {
                    true
                } else {
                    !matches!(&*prop_schema.borrow(), BoolSchema(true) | Any) && rng.gen_bool(0.7)
                };
                if include {
                    let val = generate_value(prop_schema, rng, depth.saturating_sub(1));
                    map.insert(k.clone(), val);
                }
            }

            let min_p: usize = min_properties.unwrap_or(0);
            let max_p: usize = max_properties.unwrap_or(usize::MAX);

            if !matches!(&*additional.borrow(), BoolSchema(false))
                && !matches!(satisfiability(additional), Satisfiability::Never)
            {
                // If we need to hit `minProperties`, keep inventing additional keys
                // until we reach the minimum before attempting the probabilistic
                // extras.
                while map.len() < min_p {
                    let key = random_key(rng);
                    if map.contains_key(&key) {
                        continue;
                    }
                    let val = generate_value(additional, rng, depth.saturating_sub(1));
                    map.insert(key, val);
                }

                if rng.gen_bool(0.3) {
                    let mut attempts = 0;
                    while rng.gen_bool(0.5) && (map.len() < max_p) && attempts < 5 {
                        let key = random_key(rng);
                        if map.contains_key(&key) {
                            attempts += 1;
                            continue;
                        }
                        let val = generate_value(additional, rng, depth.saturating_sub(1));
                        map.insert(key, val);
                        attempts += 1;
                    }
                }
            }

            if map.len() < min_p {
                for (k, prop_schema) in properties {
                    if map.contains_key(k) {
                        continue;
                    }
                    let val = generate_value(prop_schema, rng, depth.saturating_sub(1));
                    map.insert(k.clone(), val);
                    if map.len() >= min_p {
                        break;
                    }
                }
            }

            if map.is_empty() && min_p > 0 && !properties.is_empty() {
                if let Some((k, schema)) = properties.iter().next() {
                    let val = generate_value(schema, rng, depth.saturating_sub(1));
                    map.insert(k.clone(), val);
                }
            }

            Value::Object(map)
        }

        Array {
            prefix_items,
            items,
            min_items,
            max_items,
            contains,
            min_contains,
            max_contains,
            unique_items,
            enumeration,
            ..
        } => {
            if let Some(e) = enumeration {
                if !e.is_empty() {
                    let idx = rng.gen_range(0..e.len());
                    return e[idx].clone();
                }
            }
            let items_sat = satisfiability(items);
            let contains_validator = contains
                .as_ref()
                .and_then(|s| json_schema_ast::compile(&s.to_json()).ok());
            let required_contains = match (contains.as_ref(), min_contains) {
                (Some(_), Some(v)) => *v as usize,
                (Some(_), None) => 1,
                _ => 0,
            };
            let max_allowed_contains = max_contains.map(|v| v as usize);

            let mut min_len = min_items.unwrap_or(0) as usize;
            min_len = min_len.max(required_contains);

            let mut max_len = max_items
                .map(|v| v as usize)
                .unwrap_or(min_len.saturating_add(5))
                .max(min_len);
            if matches!(items_sat, Satisfiability::Never) {
                max_len = max_len.min(prefix_items.len());
            }
            if let Some(idx) = prefix_items
                .iter()
                .position(|s| matches!(satisfiability(s), Satisfiability::Never))
            {
                max_len = max_len.min(idx);
            }
            if max_len < min_len {
                min_len = max_len;
            }
            let upper = max_len.min(min_len.saturating_add(5));
            let mut length = if min_len <= upper {
                rng.gen_range(min_len..=upper)
            } else {
                min_len
            };

            let mut contains_count = 0usize;
            let mut arr = Vec::new();
            let push_value = |val: Value, arr: &mut Vec<Value>, contains_count: &mut usize| {
                if let Some(vld) = &contains_validator {
                    if vld.is_valid(&val) {
                        *contains_count += 1;
                    }
                }
                arr.push(val);
            };

            let prefix_len = prefix_items.len().min(length);
            for schema in prefix_items.iter().take(prefix_len) {
                let mut candidate = generate_array_member(schema, rng, depth.saturating_sub(1));
                if *unique_items {
                    for _ in 0..10 {
                        if !arr.contains(&candidate) {
                            break;
                        }
                        candidate = generate_array_member(schema, rng, depth.saturating_sub(1));
                    }
                    if arr.contains(&candidate) {
                        if let Value::Bool(b) = candidate {
                            candidate = Value::Bool(!b);
                        }
                    }
                }
                if let Some(limit) = max_allowed_contains {
                    if contains_count >= limit {
                        if let Some(vld) = &contains_validator {
                            let mut attempts = 0;
                            while attempts < 5 && vld.is_valid(&candidate) {
                                candidate = generate_value(schema, rng, depth.saturating_sub(1));
                                attempts += 1;
                            }
                        }
                    }
                }
                push_value(candidate, &mut arr, &mut contains_count);
            }

            if contains_count < required_contains {
                let needed = required_contains - contains_count;
                length = length.max(arr.len() + needed);
                length = length.min(max_len);
            }

            if let Some(c_schema) = contains {
                while contains_count < required_contains && arr.len() < length {
                    let mut candidate =
                        generate_array_member(c_schema, rng, depth.saturating_sub(1));
                    if *unique_items {
                        for _ in 0..10 {
                            if !arr.contains(&candidate) {
                                break;
                            }
                            candidate =
                                generate_array_member(c_schema, rng, depth.saturating_sub(1));
                        }
                        if arr.contains(&candidate) {
                            if let Value::Bool(b) = candidate {
                                candidate = Value::Bool(!b);
                            }
                        }
                    }
                    push_value(candidate, &mut arr, &mut contains_count);
                }
            }

            while arr.len() < length {
                let source_schema = if arr.len() < prefix_items.len() {
                    &prefix_items[arr.len()]
                } else {
                    items
                };
                let mut candidate = if matches!(items_sat, Satisfiability::Never)
                    && arr.len() >= prefix_items.len()
                {
                    random_any(rng, depth.saturating_sub(1))
                } else {
                    generate_array_member(source_schema, rng, depth.saturating_sub(1))
                };

                if *unique_items {
                    for _ in 0..10 {
                        if !arr.contains(&candidate) {
                            break;
                        }
                        candidate =
                            generate_array_member(source_schema, rng, depth.saturating_sub(1));
                    }
                    if arr.contains(&candidate) {
                        if let Value::Bool(b) = candidate {
                            candidate = Value::Bool(!b);
                        }
                    }
                }

                if let Some(limit) = max_allowed_contains {
                    if contains_count >= limit {
                        if let Some(vld) = &contains_validator {
                            let mut attempts = 0;
                            while attempts < 5 && vld.is_valid(&candidate) {
                                candidate = generate_value_not(
                                    contains.as_ref().unwrap_or(items),
                                    rng,
                                    depth.saturating_sub(1),
                                );
                                attempts += 1;
                            }
                        }
                    }
                }

                push_value(candidate, &mut arr, &mut contains_count);
            }
            Value::Array(arr)
        }

        Ref(_) => random_any(rng, depth),
        Defs(_) => Value::Null,
        IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } => {
            let if_validator = json_schema_ast::compile(&if_schema.to_json()).ok();

            if let Some(e) = else_schema {
                for _ in 0..16 {
                    let candidate = generate_value(e, rng, depth.saturating_sub(1));
                    if let Some(vld) = &if_validator {
                        if vld.is_valid(&candidate) {
                            continue;
                        }
                    }
                    if if_validator
                        .as_ref()
                        .map(|v| !v.is_valid(&candidate))
                        .unwrap_or(true)
                    {
                        return candidate;
                    }
                }
            }

            if let Some(t) = then_schema {
                let combined =
                    SchemaNode::new(SchemaNodeKind::AllOf(vec![if_schema.clone(), t.clone()]));
                return generate_value(&combined, rng, depth.saturating_sub(1));
            }

            generate_value(if_schema, rng, depth.saturating_sub(1))
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
        _ => Value::Null,
    }
}

/// Generate a *random JSON Schema* (subset) for fuzzing the value‑generator
/// itself.  The result is raw JSON so it can immediately be passed into the
/// authoritative validator for cross‑checking.
pub fn random_schema(rng: &mut impl Rng, depth: u8) -> Value {
    if depth == 0 {
        return Value::Bool(true);
    }
    match rng.gen_range(0..=4) {
        // Primitive types --------------------------------------------------
        0 => {
            // strings with optional minLength / maxLength
            let mut obj = Map::new();
            obj.insert("type".into(), Value::String("string".into()));
            if rng.gen_bool(0.5) {
                obj.insert("minLength".into(), rng.gen_range(0..5u64).into());
            }
            if rng.gen_bool(0.5) {
                obj.insert("maxLength".into(), rng.gen_range(5..10u64).into());
            }
            Value::Object(obj)
        }
        1 => {
            // integer range
            let mut obj = Map::new();
            obj.insert("type".into(), Value::String("integer".into()));
            let min = rng.gen_range(-20..20);
            let max = min + rng.gen_range(0..20);
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
            if rng.gen_bool(0.5) {
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
fn random_any(rng: &mut impl Rng, depth: u8) -> Value {
    match rng.gen_range(0..5) {
        0 => Value::Null,
        1 => Value::Bool(rng.gen_bool(0.5)),
        2 => {
            let n: f64 = rng.gen_range(-10.0..10.0);
            Value::Number(serde_json::Number::from_f64(n).unwrap_or_else(|| 0.into()))
        }
        3 => Value::String(random_string(rng, 1..8)),
        _ => {
            if depth > 1 && rng.gen_bool(0.5) {
                let len = rng.gen_range(0..=2);
                let mut arr = Vec::new();
                for _ in 0..len {
                    arr.push(random_any(rng, depth - 1));
                }
                Value::Array(arr)
            } else {
                Value::Object(Map::new())
            }
        }
    }
}

fn generate_array_member(schema: &SchemaNode, rng: &mut impl Rng, depth: u8) -> Value {
    match &*schema.borrow() {
        SchemaNodeKind::Any | SchemaNodeKind::BoolSchema(true) => {
            let n: i64 = rng.gen_range(0..=10);
            Value::Number(n.into())
        }
        _ => generate_value(schema, rng, depth),
    }
}

fn generate_value_not(schema: &SchemaNode, rng: &mut impl Rng, depth: u8) -> Value {
    use SchemaNodeKind::*;

    if depth == 0 {
        return Value::Null;
    }

    match &*schema.borrow() {
        BoolSchema(false) => random_any(rng, depth),
        BoolSchema(true) | Any => Value::Null,
        String { .. } => Value::Number(0.into()),
        Number { .. } | Integer { .. } => Value::String(random_string(rng, 3..8)),
        Boolean { .. } => Value::String("not-a-bool".into()),
        Null { .. } => Value::Bool(true),
        Object { .. } => Value::String(random_string(rng, 3..6)),
        Array { .. } => Value::Object(Map::new()),
        AllOf(subs) => subs
            .first()
            .map(|s| generate_value_not(s, rng, depth - 1))
            .unwrap_or_else(|| random_any(rng, depth)),
        AnyOf(subs) => {
            // A value that invalidates all branches suffices for `not anyOf`.
            if let Some(sub) = subs.first() {
                generate_value_not(sub, rng, depth - 1)
            } else {
                random_any(rng, depth)
            }
        }
        OneOf(subs) => {
            if subs.len() >= 2 {
                // Make two branches true by reusing the same valid value.
                generate_value(subs.first().unwrap(), rng, depth - 1)
            } else {
                subs.first()
                    .map(|s| generate_value_not(s, rng, depth - 1))
                    .unwrap_or_else(|| random_any(rng, depth))
            }
        }
        Not(sub) => generate_value(sub, rng, depth - 1),
        IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } => {
            // Make the condition true but violate the `then` branch when present.
            if let Some(t) = then_schema {
                let mut candidate = generate_value(if_schema, rng, depth - 1);
                if let Value::Object(ref mut obj) = candidate {
                    obj.insert(random_key(rng), generate_value_not(t, rng, depth - 1));
                }
                candidate
            } else if let Some(e) = else_schema {
                generate_value(e, rng, depth - 1)
            } else {
                random_any(rng, depth)
            }
        }
        Const(v) => {
            if v.is_number() {
                Value::String(random_string(rng, 2..6))
            } else {
                Value::Number(0.into())
            }
        }
        Ref(_) | Defs(_) => random_any(rng, depth),

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
        _ => Value::Null,
    }
}

fn random_key(rng: &mut impl Rng) -> String {
    random_string(rng, 3..8)
}

fn random_string(rng: &mut impl Rng, len_range: std::ops::Range<usize>) -> String {
    let len = rng.gen_range(len_range);
    (0..len)
        .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
        .collect()
}

// lots to improve here:
// - anyof generation
// - better random any
// - better not handling
// - lots of other stuff

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};
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

    fn build_required_object_schema() -> SchemaNode {
        let raw = json!({
            "type": "object",
            "properties": {
                "class": {"const": "MyClass"},
                "opt": {"type": "integer"}
            },
            "required": ["class"]
        });
        json_schema_ast::build_and_resolve_schema(&raw).unwrap()
    }
}

use json_schema_ast::{SchemaNode, SchemaNodeKind};
use rand::Rng;
use serde_json::{Map, Value};

/// Generate a random JSON value *intended* to satisfy `schema`.
/// We limit recursion with `depth`.
pub fn generate_value(schema: &SchemaNode, rng: &mut impl Rng, depth: u8) -> Value {
    if depth == 0 {
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

            for sub in subs {
                if matches!(&*sub.borrow(), BoolSchema(true) | Any) {
                    continue;
                }
                return generate_value(sub, rng, depth.saturating_sub(1));
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

        Not(_) => random_any(rng, depth),

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

            if !matches!(&*additional.borrow(), BoolSchema(false)) {
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
            items,
            min_items,
            max_items,
            contains,
            enumeration,
            ..
        } => {
            if let Some(e) = enumeration {
                if !e.is_empty() {
                    let idx = rng.gen_range(0..e.len());
                    return e[idx].clone();
                }
            }
            let base_min = if contains.is_some() {
                min_items.unwrap_or(0).max(1)
            } else {
                min_items.unwrap_or(0)
            };

            let max_i = max_items.unwrap_or(base_min + 5).max(base_min);
            let length = rng.gen_range(base_min..=max_i.min(base_min + 5));

            let mut arr = Vec::new();
            if let Some(c_schema) = contains {
                let v = generate_value(c_schema, rng, depth.saturating_sub(1));
                arr.push(v);
            }

            while arr.len() < length as usize {
                let v = generate_value(items, rng, depth.saturating_sub(1));
                arr.push(v);
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
            if rng.gen_bool(0.5) {
                if let Some(t) = then_schema {
                    return generate_value(t, rng, depth.saturating_sub(1));
                }
            }
            if let Some(e) = else_schema {
                generate_value(e, rng, depth.saturating_sub(1))
            } else {
                generate_value(if_schema, rng, depth.saturating_sub(1))
            }
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
fn random_any(_rng: &mut impl Rng, _depth: u8) -> Value {
    // Always return an empty object – this is valid under the Draft 2020‑12
    // meta‑schema as well as under any unconstrained (`true`) schema.
    Value::Object(Map::new())
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

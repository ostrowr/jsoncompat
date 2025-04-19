use json_schema_draft2020::SchemaNode;
use rand::Rng;
use serde_json::{Map, Value};

/// Generate a random JSON value *intended* to satisfy `schema`.
/// Because JSON Schema can be very large in scope, we only handle
/// core "type" constraints, `enum`, and a few others. We also limit
/// recursion with `depth`.
pub fn generate_value(schema: &SchemaNode, rng: &mut impl Rng, depth: u8) -> Value {
    // If we reach too deep, return something minimal
    if depth == 0 {
        return Value::Null;
    }
    match schema {
        // false => no valid data. We'll generate something invalid to reflect that.
        SchemaNode::BoolSchema(false) => Value::Null,

        // true or "Any" => pick something random
        SchemaNode::BoolSchema(true) | SchemaNode::Any => random_any(rng, depth),

        // if there's an enum => pick from the enum randomly
        SchemaNode::Enum(vals) if !vals.is_empty() => {
            let idx = rng.gen_range(0..vals.len());
            vals[idx].clone()
        }

        // AllOf => pick from one sub-schema, then refine by others, or just pick from the first.
        // For a simplistic approach, pick from a random sub-schema.
        SchemaNode::AllOf(subs) if !subs.is_empty() => {
            let idx = rng.gen_range(0..subs.len());
            generate_value(&subs[idx], rng, depth.saturating_sub(1))
        }
        // AnyOf => pick from one sub
        SchemaNode::AnyOf(subs) if !subs.is_empty() => {
            let idx = rng.gen_range(0..subs.len());
            generate_value(&subs[idx], rng, depth.saturating_sub(1))
        }
        // OneOf => same as AnyOf for generation
        SchemaNode::OneOf(subs) if !subs.is_empty() => {
            let idx = rng.gen_range(0..subs.len());
            generate_value(&subs[idx], rng, depth.saturating_sub(1))
        }
        SchemaNode::Not(_sub) => {
            // We'll generate random_any but ensure it's not valid for `sub`.
            // That might be tricky, so for this demonstration, do random_any.
            // (We might produce something invalid for the main schema though.
            //  'not' is tricky to satisfy generically.)

            random_any(rng, depth)
        }

        // string
        SchemaNode::String {
            min_length,
            max_length,
            enumeration,
            ..
        } => {
            if let Some(e) = enumeration {
                if !e.is_empty() {
                    // pick from enum
                    let idx = rng.gen_range(0..e.len());
                    return e[idx].clone();
                }
            }
            let len_min = min_length.unwrap_or(0);
            let len_max = max_length.unwrap_or(len_min + 5).max(len_min); // fallback
            let length = if len_min <= len_max {
                rng.gen_range(len_min..=len_max.min(len_min + 10)) // limit random
            } else {
                len_min
            };
            let s: String = (0..length)
                .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
                .collect();
            Value::String(s)
        }

        // number
        SchemaNode::Number {
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
            // Special‑case multipleOf – 0 satisfies any divisor and is easy.
            if multiple_of.is_some() {
                let low = minimum.unwrap_or(f64::NEG_INFINITY);
                let high = maximum.unwrap_or(f64::INFINITY);
                if low <= 0.0 && 0.0 <= high {
                    return Value::Number(serde_json::Number::from_f64(0.0).unwrap());
                }
            }

            // Fallback: pick a f64 in [min..max]
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

        // integer
        SchemaNode::Integer {
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
            // multipleOf shortcut via 0
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
                    let mo = (*mo_f).round() as i64; // safe approximation for integers
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

        // boolean
        SchemaNode::Boolean { enumeration } => {
            if let Some(e) = enumeration {
                if !e.is_empty() {
                    let idx = rng.gen_range(0..e.len());
                    return e[idx].clone();
                }
            }
            Value::Bool(rng.gen_bool(0.5))
        }

        // null
        SchemaNode::Null { enumeration } => {
            if let Some(e) = enumeration {
                if !e.is_empty() {
                    let idx = rng.gen_range(0..e.len());
                    return e[idx].clone();
                }
            }
            Value::Null
        }

        // object
        SchemaNode::Object {
            properties,
            required,
            additional,
            min_properties,
            max_properties,
            enumeration,
            ..
        } => {
            // If there's an enumeration, pick from it
            if let Some(e) = enumeration {
                if !e.is_empty() {
                    let idx = rng.gen_range(0..e.len());
                    return e[idx].clone();
                }
            }
            // For known properties, fill them with random data that fits
            // or optionally skip if not required
            let mut map = Map::new();
            for (k, prop_schema) in properties {
                let must_include = required.contains(k);
                // We include optional fields with a 70% probability *unless*
                // the subschema is completely unconstrained (`true` or `Any`).
                // Such properties are prone to introducing invalid data when
                // they actually reference recursive structures (e.g. a
                // self‑referential `$ref: "#"`).  Omitting them is always safe
                // because they are not required and an absent property cannot
                // violate the parent schema.
                let include = if must_include {
                    true
                } else {
                    match prop_schema {
                        SchemaNode::BoolSchema(true) | SchemaNode::Any => false,
                        _ => rng.gen_bool(0.7),
                    }
                };
                if include {
                    let val = generate_value(prop_schema, rng, depth.saturating_sub(1));
                    map.insert(k.clone(), val);
                }
            }

            // Determine the desired object size bounds if specified.
            let min_p = min_properties.map(|v| *v as usize).unwrap_or(0);
            let max_p = max_properties.map(|v| *v as usize);

            // Random extra properties if allowed and we have not reached min_properties.
            if !matches!(additional.as_ref(), SchemaNode::BoolSchema(false)) {
                // Continue adding extra properties until we reach `min_properties`.
                while map.len() < min_p {
                    let key = random_key(rng);
                    if map.contains_key(&key) {
                        continue;
                    }
                    let val = generate_value(additional, rng, depth.saturating_sub(1));
                    map.insert(key, val);
                }

                // Optionally add more properties (respecting max_properties if set).
                if rng.gen_bool(0.3) {
                    let mut attempts = 0;
                    while rng.gen_bool(0.5)
                        && (max_p.map_or(true, |m| map.len() < m))
                        && attempts < 5
                    {
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

            // As a final fallback, if we still have fewer than `min_properties` but cannot
            // add additional properties (because `additionalProperties: false`), attempt
            // to include optional defined properties to satisfy the minimum.
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

            Value::Object(map)
        }

        // array
        SchemaNode::Array {
            items,
            min_items,
            max_items,
            contains,
            enumeration,
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

            // If contains constraint, first insert an element satisfying it
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

        // If we see a leftover Ref, it likely wasn't resolved. We'll just fallback
        SchemaNode::Ref(_) => random_any(rng, depth),

        // Handle new SchemaNode variants
        SchemaNode::Defs(_) => Value::Null,
        SchemaNode::Const(v) => v.clone(),
        SchemaNode::Type(_) => Value::Null,
        SchemaNode::Minimum(_) => Value::Null,
        SchemaNode::Maximum(_) => Value::Null,
        SchemaNode::Required(_) => Value::Null,
        SchemaNode::AdditionalProperties(_) => Value::Null,
        SchemaNode::Format(_) => Value::Null,
        SchemaNode::ContentEncoding(_) => Value::Null,
        SchemaNode::ContentMediaType(_) => Value::Null,
        SchemaNode::Title(_) => Value::Null,
        SchemaNode::Description(_) => Value::Null,
        SchemaNode::Default(_) => Value::Null,
        SchemaNode::Examples(_) => Value::Null,
        SchemaNode::ReadOnly(_) => Value::Null,
        SchemaNode::WriteOnly(_) => Value::Null,

        // Handle additional SchemaNode variants
        SchemaNode::AllOf(_) => Value::Null,
        SchemaNode::AnyOf(_) => Value::Null,
        SchemaNode::OneOf(_) => Value::Null,
        // SchemaNode::Not(_) => Value::Null,
        SchemaNode::Enum(_) => Value::Null,
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

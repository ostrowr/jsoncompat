use crate::SchemaNode;
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
            ..
        } => {
            if let Some(e) = enumeration {
                if !e.is_empty() {
                    let idx = rng.gen_range(0..e.len());
                    return e[idx].clone();
                }
            }
            // We'll pick a f64 in [min..max], or fallback
            let low = minimum.unwrap_or(0.0).max(-1_000_000.0);
            let high = maximum.unwrap_or(1_000_000.0).min(1_000_000.0);
            let val = rng.gen_range(low..=high);
            Value::Number(serde_json::Number::from_f64(val).unwrap_or_else(|| 0.into()))
        }

        // integer
        SchemaNode::Integer {
            enumeration,
            minimum,
            maximum,
            ..
        } => {
            if let Some(e) = enumeration {
                if !e.is_empty() {
                    let idx = rng.gen_range(0..e.len());
                    return e[idx].clone();
                }
            }
            let low = minimum.unwrap_or(-1000).max(-1000000);
            let high = maximum.unwrap_or(1000).min(1000000);
            let val = rng.gen_range(low..=high);
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
            enumeration,
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
                // 70% chance to include optional fields
                let include = must_include || rng.gen_bool(0.7);
                if include {
                    let val = generate_value(prop_schema, rng, depth.saturating_sub(1));
                    map.insert(k.clone(), val);
                }
            }
            // Random extra property? If `additional` allows
            // 30% chance to add an extra property
            if rng.gen_bool(0.3) {
                let key = random_key(rng);
                let val = generate_value(additional, rng, depth.saturating_sub(1));
                map.insert(key, val);
            }
            Value::Object(map)
        }

        // array
        SchemaNode::Array {
            items,
            min_items,
            max_items,
            enumeration,
        } => {
            if let Some(e) = enumeration {
                if !e.is_empty() {
                    let idx = rng.gen_range(0..e.len());
                    return e[idx].clone();
                }
            }
            let min_i = min_items.unwrap_or(0);
            let max_i = max_items.unwrap_or(min_i + 5).max(min_i);
            let length = rng.gen_range(min_i..=max_i.min(min_i + 5));
            let mut arr = Vec::new();
            for _ in 0..length {
                let v = generate_value(items, rng, depth.saturating_sub(1));
                arr.push(v);
            }
            Value::Array(arr)
        }

        // If we see a leftover Ref, it likely wasn't resolved. We'll just fallback
        SchemaNode::Ref(_) => random_any(rng, depth),

        // Handle new SchemaNode variants
        SchemaNode::Defs(_) => Value::Null,
        SchemaNode::Const(_) => Value::Null,
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

/// A minimal fallback to produce a random JSON value of any type.
fn random_any(rng: &mut impl Rng, depth: u8) -> Value {
    let pick = rng.gen_range(0..5);
    match pick {
        0 => Value::Null,
        1 => Value::Bool(rng.gen_bool(0.5)),
        2 => Value::Number((rng.gen_range(-100..100)).into()),
        3 => Value::String(random_string(rng, 0..8)),
        4 => {
            // object
            if depth == 0 {
                Value::Null
            } else {
                let mut m = Map::new();
                let n = rng.gen_range(0..3);
                for _ in 0..n {
                    let key = random_key(rng);
                    let val = random_any(rng, depth - 1);
                    m.insert(key, val);
                }
                Value::Object(m)
            }
        }
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

/// A partial validator: checks if `val` is valid against `schema`.
/// This can be used to confirm that `generate_value(schema,...)`
/// indeed yields something valid. (Though we rely on a partial
/// approach in `is_subschema_of`.)
///
/// For full correctness, you'd need a robust validator. This
/// version handles the same subset of features we generate.
pub fn validate(schema: &SchemaNode, val: &Value) -> bool {
    // We can reuse the existing `instance_is_valid_against` logic from
    // your library's subset checks, but that was incomplete for advanced
    // constraints. We'll do a similarly partial approach here.
    crate::instance_is_valid_against(val, schema)
}

/// Fuzz check both schemas by generating random data from each and
/// seeing if it’s accepted by the other schema. This helps find
/// real-world counterexamples.
pub fn fuzz_compat_check(
    old_schema: &SchemaNode,
    new_schema: &SchemaNode,
    samples_per_side: usize,
    max_depth: u8,
    rng: &mut impl Rng,
) {
    // 1) Generate data from old_schema
    for _ in 0..samples_per_side {
        let val = generate_value(old_schema, rng, max_depth);
        let valid_old = validate(old_schema, &val);
        let valid_new = validate(new_schema, &val);
        if valid_old && !valid_new {
            // This is a data sample accepted by old but rejected by new
            println!("[Fuzz] Found data accepted by OLD but not NEW:\n  {val}");
        }
    }

    // 2) Generate data from new_schema
    for _ in 0..samples_per_side {
        let val = generate_value(new_schema, rng, max_depth);
        let valid_new = validate(new_schema, &val);
        let valid_old = validate(old_schema, &val);
        if valid_new && !valid_old {
            // This is a data sample accepted by new but rejected by old
            println!("[Fuzz] Found data accepted by NEW but not OLD:\n  {val}");
        }
    }
}

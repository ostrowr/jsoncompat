use json_schema_ast::{SchemaNode, SchemaNodeKind};
use rand::{seq::SliceRandom, Rng};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerateError {
    Unsatisfiable,
    Exhausted,
}

impl std::fmt::Display for GenerateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsatisfiable => f.write_str("schema cannot be satisfied"),
            Self::Exhausted => f.write_str("failed to generate a satisfying value"),
        }
    }
}

impl std::error::Error for GenerateError {}

pub type GenerateResult = Result<Value, GenerateError>;

/// Generate a random JSON value *intended* to satisfy `schema`.
/// We limit recursion with `depth`.
pub fn generate_value(schema: &SchemaNode, rng: &mut impl Rng, depth: u8) -> GenerateResult {
    if depth == 0 {
        return Err(GenerateError::Exhausted);
    }

    use SchemaNodeKind::*;

    match &*schema.borrow() {
        BoolSchema(false) => Err(GenerateError::Unsatisfiable),
        BoolSchema(true) | Any => Ok(random_any(rng, depth)),
        Enum(vals) => {
            if vals.is_empty() {
                Err(GenerateError::Unsatisfiable)
            } else {
                let idx = rng.gen_range(0..vals.len());
                Ok(vals[idx].clone())
            }
        }
        AllOf(subs) => {
            if subs.is_empty() {
                return Ok(random_any(rng, depth));
            }

            if subs
                .iter()
                .any(|s| matches!(&*s.borrow(), BoolSchema(false)))
            {
                return Err(GenerateError::Unsatisfiable);
            }

            if subs.iter().all(|s| matches!(&*s.borrow(), Object { .. })) {
                use std::collections::HashMap;

                let mut combined = Map::new();
                for sub in subs {
                    match generate_value(sub, rng, depth.saturating_sub(1))? {
                        Value::Object(obj) => {
                            for (k, v) in obj {
                                combined.insert(k, v);
                            }
                        }
                        _ => return Err(GenerateError::Exhausted),
                    }
                }

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
                    let val = generate_value(&schema, rng, depth.saturating_sub(1))?;
                    combined.insert(k, val);
                }

                Ok(Value::Object(combined))
            } else {
                let mut exhausted = false;
                for sub in subs {
                    if matches!(&*sub.borrow(), BoolSchema(true) | Any) {
                        continue;
                    }
                    match generate_value(sub, rng, depth.saturating_sub(1)) {
                        Ok(v) => return Ok(v),
                        Err(GenerateError::Unsatisfiable) => {
                            return Err(GenerateError::Unsatisfiable);
                        }
                        Err(GenerateError::Exhausted) => exhausted = true,
                    }
                }

                if exhausted {
                    return Err(GenerateError::Exhausted);
                }
                Ok(random_any(rng, depth))
            }
        }
        AnyOf(subs) => {
            if subs.is_empty() {
                return Err(GenerateError::Unsatisfiable);
            }

            let mut order: Vec<_> = (0..subs.len()).collect();
            order.shuffle(rng);

            let mut unsat = 0usize;
            for idx in order {
                match generate_value(&subs[idx], rng, depth.saturating_sub(1)) {
                    Ok(v) => return Ok(v),
                    Err(GenerateError::Unsatisfiable) => {
                        unsat += 1;
                    }
                    Err(GenerateError::Exhausted) => {}
                }
            }

            if unsat == subs.len() {
                Err(GenerateError::Unsatisfiable)
            } else {
                Err(GenerateError::Exhausted)
            }
        }
        OneOf(subs) => {
            if subs.is_empty() {
                return Err(GenerateError::Unsatisfiable);
            }

            let validators: Vec<_> = subs
                .iter()
                .map(|s| json_schema_ast::compile(&s.to_json()).ok())
                .collect();

            let mut unsat = vec![false; subs.len()];

            for _ in 0..32 {
                let pick = rng.gen_range(0..subs.len());
                match generate_value(&subs[pick], rng, depth.saturating_sub(1)) {
                    Ok(candidate) => {
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
                            return Ok(candidate);
                        }
                    }
                    Err(GenerateError::Unsatisfiable) => {
                        unsat[pick] = true;
                        if unsat.iter().all(|&b| b) {
                            return Err(GenerateError::Unsatisfiable);
                        }
                    }
                    Err(GenerateError::Exhausted) => {}
                }
            }

            if unsat.iter().all(|&b| b) {
                Err(GenerateError::Unsatisfiable)
            } else {
                Err(GenerateError::Exhausted)
            }
        }
        Not(sub) => match &*sub.borrow() {
            BoolSchema(true) | Any => Err(GenerateError::Unsatisfiable),
            _ => Ok(random_any(rng, depth)),
        },
        String {
            min_length,
            max_length,
            enumeration,
            ..
        } => {
            if let Some(e) = enumeration {
                if e.is_empty() {
                    return Err(GenerateError::Unsatisfiable);
                }
                let idx = rng.gen_range(0..e.len());
                return Ok(e[idx].clone());
            }

            let len_min = (*min_length).unwrap_or(0);
            let len_max = (*max_length).unwrap_or(len_min + 5);
            if len_max < len_min {
                return Err(GenerateError::Unsatisfiable);
            }
            let upper = len_max.min(len_min + 10);
            let length = rng.gen_range(len_min..=upper);
            let s: std::string::String = (0..length)
                .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
                .collect();
            Ok(Value::String(s))
        }
        Number {
            enumeration,
            minimum,
            maximum,
            multiple_of,
            exclusive_minimum,
            exclusive_maximum,
            ..
        } => {
            if let Some(e) = enumeration {
                if e.is_empty() {
                    return Err(GenerateError::Unsatisfiable);
                }
                let idx = rng.gen_range(0..e.len());
                return Ok(e[idx].clone());
            }

            if let (Some(min), Some(max)) = (*minimum, *maximum) {
                if min > max || (min == max && (*exclusive_minimum || *exclusive_maximum)) {
                    return Err(GenerateError::Unsatisfiable);
                }
            }

            let mut low = (*minimum).unwrap_or(-1_000_000.0);
            let mut high = (*maximum).unwrap_or(1_000_000.0);

            low = low.max(-1_000_000.0);
            high = high.min(1_000_000.0);

            if !(low.is_finite() && high.is_finite()) || low > high {
                return Err(GenerateError::Unsatisfiable);
            }

            if let Some(mo) = *multiple_of {
                if mo <= 0.0 {
                    return Err(GenerateError::Unsatisfiable);
                }
                if low <= 0.0 && 0.0 <= high {
                    let num = serde_json::Number::from_f64(0.0).ok_or(GenerateError::Exhausted)?;
                    return Ok(Value::Number(num));
                }
                let k = (low / mo).ceil();
                let candidate = k * mo;
                if candidate > high {
                    return Err(GenerateError::Unsatisfiable);
                }
                let num =
                    serde_json::Number::from_f64(candidate).ok_or(GenerateError::Exhausted)?;
                return Ok(Value::Number(num));
            }
            let mut attempts = 0;
            loop {
                let val = rng.gen_range(low..=high);
                if (*exclusive_minimum && val <= low) || (*exclusive_maximum && val >= high) {
                    attempts += 1;
                    if attempts > 32 {
                        return Err(GenerateError::Exhausted);
                    }
                    continue;
                }
                let num = serde_json::Number::from_f64(val).ok_or(GenerateError::Exhausted)?;
                break Ok(Value::Number(num));
            }
        }
        Integer {
            enumeration,
            minimum,
            maximum,
            multiple_of,
            exclusive_minimum,
            exclusive_maximum,
            ..
        } => {
            if let Some(e) = enumeration {
                if e.is_empty() {
                    return Err(GenerateError::Unsatisfiable);
                }
                let idx = rng.gen_range(0..e.len());
                return Ok(e[idx].clone());
            }

            let mut low = (*minimum).unwrap_or(-1_000_000);
            let mut high = (*maximum).unwrap_or(1_000_000);

            if *exclusive_minimum {
                low = low.checked_add(1).ok_or(GenerateError::Unsatisfiable)?;
            }
            if *exclusive_maximum {
                high = high.checked_sub(1).ok_or(GenerateError::Unsatisfiable)?;
            }

            if low > high {
                return Err(GenerateError::Unsatisfiable);
            }

            if let Some(mo_f) = *multiple_of {
                if mo_f <= 0.0 {
                    return Err(GenerateError::Unsatisfiable);
                }
                if mo_f.fract().abs() > f64::EPSILON {
                    if low <= 0 && high >= 0 {
                        return Ok(Value::Number(0.into()));
                    }
                    let mut candidate = None;
                    for k in -1_000..=1_000 {
                        let value = mo_f * k as f64;
                        let rounded = value.round();
                        if (value - rounded).abs() < 1e-6 {
                            let int_value = rounded as i64;
                            if int_value >= low && int_value <= high {
                                candidate = Some(int_value);
                                break;
                            }
                        }
                    }
                    if let Some(found) = candidate {
                        return Ok(Value::Number(found.into()));
                    }
                    return Err(GenerateError::Unsatisfiable);
                }
                let mo = mo_f.round() as i64;
                if mo == 0 {
                    if low <= 0 && high >= 0 {
                        return Ok(Value::Number(0.into()));
                    }
                    return Err(GenerateError::Unsatisfiable);
                }
                let remainder = low.rem_euclid(mo);
                let candidate = if remainder == 0 {
                    low
                } else {
                    low + (mo - remainder)
                };
                if candidate > high {
                    return Err(GenerateError::Unsatisfiable);
                }
                return Ok(Value::Number(candidate.into()));
            }

            let val = rng.gen_range(low..=high);
            Ok(Value::Number(val.into()))
        }
        Boolean { enumeration } => {
            if let Some(e) = enumeration {
                if e.is_empty() {
                    return Err(GenerateError::Unsatisfiable);
                }
                let idx = rng.gen_range(0..e.len());
                return Ok(e[idx].clone());
            }

            Ok(Value::Bool(rng.gen_bool(0.5)))
        }
        Null { enumeration } => {
            if let Some(e) = enumeration {
                if e.is_empty() {
                    return Err(GenerateError::Unsatisfiable);
                }
                let idx = rng.gen_range(0..e.len());
                return Ok(e[idx].clone());
            }
            Ok(Value::Null)
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
                if e.is_empty() {
                    return Err(GenerateError::Unsatisfiable);
                }
                let idx = rng.gen_range(0..e.len());
                return Ok(e[idx].clone());
            }

            let min_p = (*min_properties).unwrap_or(0);
            let max_p = (*max_properties).unwrap_or(usize::MAX);

            if max_p < min_p || max_p < required.len() {
                return Err(GenerateError::Unsatisfiable);
            }

            let mut map = Map::new();

            for (k, prop_schema) in properties {
                let must_include = required.contains(k);
                if map.len() >= max_p && !must_include {
                    continue;
                }
                let include = if must_include {
                    true
                } else {
                    !matches!(&*prop_schema.borrow(), BoolSchema(true) | Any) && rng.gen_bool(0.7)
                };
                if !include {
                    continue;
                }
                match generate_value(prop_schema, rng, depth.saturating_sub(1)) {
                    Ok(val) => {
                        if map.len() >= max_p && !must_include {
                            continue;
                        }
                        map.insert(k.clone(), val);
                    }
                    Err(GenerateError::Unsatisfiable) => {
                        if must_include {
                            return Err(GenerateError::Unsatisfiable);
                        }
                    }
                    Err(GenerateError::Exhausted) => {
                        if must_include {
                            return Err(GenerateError::Exhausted);
                        }
                    }
                }
            }

            if !matches!(&*additional.borrow(), BoolSchema(false)) {
                while map.len() < min_p {
                    if map.len() >= max_p {
                        break;
                    }
                    let key = random_key(rng);
                    if map.contains_key(&key) {
                        continue;
                    }
                    match generate_value(additional, rng, depth.saturating_sub(1)) {
                        Ok(val) => {
                            map.insert(key, val);
                        }
                        Err(GenerateError::Unsatisfiable) => {
                            return Err(GenerateError::Unsatisfiable);
                        }
                        Err(GenerateError::Exhausted) => {
                            return Err(GenerateError::Exhausted);
                        }
                    }
                }

                if rng.gen_bool(0.3) {
                    let mut attempts = 0;
                    while rng.gen_bool(0.5) && map.len() < max_p && attempts < 5 {
                        let key = random_key(rng);
                        if map.contains_key(&key) {
                            attempts += 1;
                            continue;
                        }
                        match generate_value(additional, rng, depth.saturating_sub(1)) {
                            Ok(val) => {
                                map.insert(key, val);
                            }
                            Err(GenerateError::Unsatisfiable) => break,
                            Err(GenerateError::Exhausted) => {
                                attempts += 1;
                                continue;
                            }
                        }
                        attempts += 1;
                    }
                }
            }

            if map.len() < min_p {
                for (k, prop_schema) in properties {
                    if map.contains_key(k) {
                        continue;
                    }
                    if map.len() >= max_p {
                        break;
                    }
                    let val = generate_value(prop_schema, rng, depth.saturating_sub(1))?;
                    map.insert(k.clone(), val);
                    if map.len() >= min_p {
                        break;
                    }
                }
            }

            if map.is_empty() && min_p > 0 && !properties.is_empty() {
                if let Some((k, schema)) = properties.iter().next() {
                    if !map.contains_key(k) {
                        let val = generate_value(schema, rng, depth.saturating_sub(1))?;
                        map.insert(k.clone(), val);
                    }
                }
            }

            if map.len() < min_p {
                return Err(GenerateError::Exhausted);
            }
            if map.len() > max_p {
                return Err(GenerateError::Unsatisfiable);
            }

            Ok(Value::Object(map))
        }
        Array {
            items,
            min_items,
            max_items,
            contains,
            enumeration,
        } => {
            if let Some(e) = enumeration {
                if e.is_empty() {
                    return Err(GenerateError::Unsatisfiable);
                }
                let idx = rng.gen_range(0..e.len());
                return Ok(e[idx].clone());
            }
            let mut min_len = (*min_items).unwrap_or(0);
            if contains.is_some() {
                min_len = min_len.max(1);
            }

            let mut max_len = (*max_items).unwrap_or(min_len + 5);
            if max_len < min_len {
                return Err(GenerateError::Unsatisfiable);
            }
            max_len = max_len.max(min_len);
            let upper = max_len.min(min_len + 5);
            let length = rng.gen_range(min_len..=upper);

            let mut arr = Vec::new();
            if let Some(c_schema) = contains {
                match generate_value(c_schema, rng, depth.saturating_sub(1)) {
                    Ok(v) => arr.push(v),
                    Err(GenerateError::Unsatisfiable) => {
                        return Err(GenerateError::Unsatisfiable);
                    }
                    Err(GenerateError::Exhausted) => {
                        return Err(GenerateError::Exhausted);
                    }
                }
            }

            while arr.len() < length as usize {
                match generate_value(items, rng, depth.saturating_sub(1)) {
                    Ok(v) => arr.push(v),
                    Err(GenerateError::Unsatisfiable) => {
                        return Err(GenerateError::Unsatisfiable);
                    }
                    Err(GenerateError::Exhausted) => {
                        return Err(GenerateError::Exhausted);
                    }
                }
            }

            Ok(Value::Array(arr))
        }
        Ref(_) => Ok(random_any(rng, depth)),
        Defs(_) => Err(GenerateError::Exhausted),
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
        Const(v) => Ok(v.clone()),
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
        | WriteOnly(_) => Err(GenerateError::Exhausted),
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
            let value = generate_value(&schema, &mut rng, 5)
                .expect("expected generator to produce a value for required object schema");
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

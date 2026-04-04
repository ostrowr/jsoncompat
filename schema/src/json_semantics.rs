use fancy_regex::Regex;
use serde_json::Value;

#[must_use]
pub fn json_values_equal(expected: &Value, value: &Value) -> bool {
    match (expected, value) {
        (Value::Number(_), Value::Number(_)) => numeric_values_equal(expected, value),
        (Value::Array(expected_items), Value::Array(items)) => {
            expected_items.len() == items.len()
                && expected_items
                    .iter()
                    .zip(items)
                    .all(|(expected, value)| json_values_equal(expected, value))
        }
        (Value::Object(expected_object), Value::Object(object)) => {
            expected_object.len() == object.len()
                && expected_object.iter().all(|(key, expected)| {
                    object
                        .get(key)
                        .is_some_and(|value| json_values_equal(expected, value))
                })
        }
        _ => expected == value,
    }
}

#[must_use]
pub fn property_name_matches_pattern(pattern: &str, property_name: &str) -> bool {
    Regex::new(pattern)
        .ok()
        .and_then(|regex| regex.is_match(property_name).ok())
        .unwrap_or(false)
}

pub(crate) fn numeric_values_equal(expected: &Value, value: &Value) -> bool {
    if let (Some(expected_integer), Some(value_integer)) = (
        integer_value_from_json(expected),
        integer_value_from_json(value),
    ) {
        return expected_integer == value_integer;
    }

    expected
        .as_f64()
        .zip(value.as_f64())
        .is_some_and(|(expected_number, actual_number)| expected_number == actual_number)
}

pub(crate) fn integer_value_from_json(value: &Value) -> Option<i128> {
    let Value::Number(number) = value else {
        return None;
    };

    number
        .as_i64()
        .map(i128::from)
        .or_else(|| number.as_u64().map(i128::from))
        .or_else(|| number.as_f64().and_then(integer_value_from_f64))
}

fn integer_value_from_f64(value: f64) -> Option<i128> {
    if !value.is_finite() || value.fract() != 0.0 {
        return None;
    }

    let integer = value as i128;
    ((integer as f64) == value).then_some(integer)
}

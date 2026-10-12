//! The default deliberately follows serde_json's bounded numeric representation.
//! Run separately from all-features tests so feature unification cannot hide it.
use jsoncompat::{Role, SchemaDocument, check_compat};
use serde_json::{Value, json};

#[test]
fn bounded_integer_values_remain_exact_in_both_modes() {
    let value: Value = serde_json::from_str("18446744073709551615").unwrap();
    assert_eq!(value.as_u64(), Some(u64::MAX));
    assert_eq!(value.to_string(), u64::MAX.to_string());
    let schema = SchemaDocument::from_json(&json!({"const": value})).unwrap();
    assert!(schema.is_valid(&json!(u64::MAX)).unwrap());
    assert!(!schema.is_valid(&json!(u64::MAX - 1)).unwrap());
}

#[test]
fn decimal_rounding_is_an_explicit_feature_tradeoff() {
    let precise: Value = serde_json::from_str("0.10000000000000001").unwrap();
    let rounded = json!(0.1);
    let schema = SchemaDocument::from_json(&json!({"const": precise})).unwrap();
    let other = SchemaDocument::from_json(&json!({"const": rounded})).unwrap();
    let distinct = cfg!(feature = "arbitrary-precision");
    assert_eq!(schema.is_valid(&rounded).unwrap(), !distinct);
    assert_eq!(
        check_compat(&schema, &other, Role::Both).unwrap(),
        !distinct
    );
    assert_eq!(
        precise.to_string(),
        if distinct {
            "0.10000000000000001"
        } else {
            "0.1"
        }
    );
}

#[test]
fn exponent_range_follows_the_selected_representation() {
    let tiny: Value = serde_json::from_str("1e-400").unwrap();
    let huge = serde_json::from_str::<Value>("1e400");
    let zero = SchemaDocument::from_json(&json!({"const": 0})).unwrap();
    if cfg!(feature = "arbitrary-precision") {
        assert_eq!(tiny.to_string(), "1e-400");
        assert_eq!(huge.unwrap().to_string(), "1e+400");
        assert!(!zero.is_valid(&tiny).unwrap());
    } else {
        assert_eq!(tiny, json!(0.0));
        assert!(huge.is_err());
        assert!(zero.is_valid(&tiny).unwrap());
    }
}

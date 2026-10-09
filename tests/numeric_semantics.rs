//! Decimal lexical regressions and metamorphic schema checks. The expected
//! signs, integrality and divisibility are mathematical labels, not another
//! invocation of either production validator.
use jsoncompat_codegen::prepared_schema::{PreparedSchema, instance::InstanceRef};
use serde_json::{Value, json};

fn verify(schema: &Value, value: &Value, expected: bool) {
    let raw = json_schema_ast::compile(schema).unwrap();
    assert_eq!(
        raw.is_valid(value),
        expected,
        "raw schema={schema}, value={value}"
    );
    let prepared = PreparedSchema::compile(schema).unwrap();
    assert_eq!(
        prepared.is_valid_json_view(InstanceRef::from_serde(value)),
        expected,
        "prepared schema={schema}, value={value}"
    );
    let loaded = PreparedSchema::load(&prepared.to_bytes().unwrap()).unwrap();
    assert_eq!(
        loaded.is_valid_json_view(InstanceRef::from_serde(value)),
        expected,
        "loaded schema={schema}, value={value}"
    );
}

#[test]
fn exponent_extremes_cannot_invert_numeric_assertions() {
    for exponent in [
        "324",
        "10000",
        "10001",
        "2147483648",
        "99999999999999999999999",
    ] {
        for sign in ["", "-"] {
            for small in [false, true] {
                let value: Value = serde_json::from_str(&format!(
                    "{sign}1e{}{exponent}",
                    if small { "-" } else { "" }
                ))
                .unwrap();
                let positive = sign.is_empty();
                for (schema, expected) in [
                    (json!({"minimum": 0}), positive),
                    (json!({"exclusiveMaximum": 0}), !positive),
                    (json!({"type":"integer"}), !small),
                    (json!({"multipleOf":2}), !small),
                    (json!({"multipleOf":3}), false),
                    (json!({"const":0}), false),
                    (json!({"enum":[0,1,-1]}), false),
                ] {
                    verify(&schema, &value, expected);
                    verify(&json!({"not":schema}), &value, !expected);
                    verify(&json!({"not":{"not":schema}}), &value, expected);
                    verify(&json!({"allOf":[true,schema]}), &value, expected);
                    verify(&json!({"anyOf":[false,schema]}), &value, expected);
                    verify(
                        &json!({"if":schema,"then":true,"else":false}),
                        &value,
                        expected,
                    );
                    verify(
                        &json!({"$defs":{"test":schema},"$ref":"#/$defs/test"}),
                        &value,
                        expected,
                    );
                }
            }
        }
    }
}

#[test]
fn exact_equality_and_divisibility_survive_nested_containers() {
    for (left, right, equal) in [
        ("1e-2147483648", "10e-2147483649", true),
        ("1e2147483648", "10e2147483647", true),
        ("1e-2147483648", "0", false),
        ("1e2147483648", "2e2147483648", false),
        ("9007199254740993", "9007199254740992", false),
    ] {
        let a: Value = serde_json::from_str(left).unwrap();
        let b: Value = serde_json::from_str(right).unwrap();
        verify(
            &json!({"const":{"items":[a.clone()]}}),
            &json!({"items":[b.clone()]}),
            equal,
        );
        verify(
            &json!({"uniqueItems":true}),
            &json!([{"n":a}, {"n":b}]),
            !equal,
        );
    }
}

#[test]
fn numeric_schema_bounds_never_collapse_in_the_proof_graph() {
    for bound in ["1e-10001", "1e2147483648", "0.10000000000000001"] {
        let number: Value = serde_json::from_str(bound).unwrap();
        let raw = json!({"type":"number","minimum":number});
        let schema = json_schema_ast::SchemaDocument::from_json(&raw).unwrap();
        for value in [json!(0), json!(1), json!(0.1), number.clone()] {
            assert_eq!(
                schema.root().unwrap().accepts_value(&value),
                schema.is_valid(&value).unwrap(),
                "bound={bound}, value={value}"
            );
            verify(&raw, &value, schema.is_valid(&value).unwrap());
        }
        let zero =
            json_schema_ast::SchemaDocument::from_json(&json!({"type":"number","minimum":0}))
                .unwrap();
        assert!(!jsoncompat::check_compat(&schema, &zero, jsoncompat::Role::Serializer).unwrap());
    }
}

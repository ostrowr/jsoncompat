use json_schema_fuzz::{generate_value, validate};
use json_schema_draft2020::{build_and_resolve_schema, compile};
use rand::{rngs::StdRng, SeedableRng};
use serde_json::json;
use url::Url;

#[test]
fn fuzz_generation_is_valid() {
    let raw = json!({
        "type": "object",
        "properties": {
            "name": {"type":"string"},
            "id": {"type":"integer", "minimum":1}
        },
        "required": ["id"]
    });
    let base = Url::parse("file:///fuzz.json").unwrap();
    let schema = build_and_resolve_schema(&raw, &base).unwrap();
    let compiled = compile(&schema.to_json()).unwrap();

    let mut rng = StdRng::seed_from_u64(42);
    for _ in 0..200 {
        let val = generate_value(&schema, &mut rng, 4);
        assert!(compiled.is_valid(&val), "generated value should be valid: {val}");
        assert!(validate(&schema, &val));
    }
}

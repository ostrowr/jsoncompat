//! Fixed corpus of more complex schemas to ensure the value generator keeps
//! working as capabilities expand.  Each schema is validated with the
//! reference implementation and then fed into the generator for 50 samples.

use json_schema_draft2020::{build_and_resolve_schema, compile};
use json_schema_fuzz::{generate_value};
use rand::{rngs::StdRng, SeedableRng};
use serde_json::json;
use url::Url;

fn corpus() -> Vec<serde_json::Value> {
    vec![
        // 1. simple string with pattern & length
        json!({"type":"string","minLength":2,"maxLength":10}),

        // 2. bounded number with exclusiveMaximum as numeric (draft 2020‑12)
        json!({"type":"number","minimum":0,"exclusiveMaximum":5}),

        // 3. integer enum
        json!({"type":"integer","enum":[1,2,3]}),

        // 4. object with required id and optional name
        json!({
            "type":"object",
            "properties":{
                "id":{"type":"integer"},
                "name":{"type":"string"}
            },
            "required":["id"],
            "additionalProperties":false
        }),

        // 5. nested object
        json!({
            "type":"object",
            "properties":{
                "user": {
                    "type":"object",
                    "properties": {
                        "first": {"type":"string"},
                        "last":  {"type":"string"}
                    },
                    "required":["first","last"]
                }
            },
            "required":["user"]
        }),

        // 6. array of integers length 3..5
        json!({
            "type":"array",
            "items":{"type":"integer"},
            "minItems":3,
            "maxItems":5
        }),

        // 7. anyOf string or number
        json!({"anyOf":[{"type":"string"},{"type":"number"}]}),

        // 8. oneOf specialised strings
        json!({"oneOf":[{"enum":["a","b"]},{"enum":["c"]}]}),

        // 9. allOf integer 0..10 AND 5..15  ⇒ 5..10
        json!({"allOf":[{"type":"integer","minimum":0,"maximum":10},{"minimum":5,"maximum":15}]}),

        //10. object with nested array property
        json!({
            "type":"object",
            "properties": {
                "tags": {"type":"array","items":{"type":"string"}}
            },
            "required":["tags"]
        }),

        //11. array of objects
        json!({
            "type":"array",
            "items": {
                "type":"object",
                "properties": {"id":{"type":"integer"}},
                "required":["id"]
            }
        }),

        //12. Enum with heterogeneous values
        json!({"enum":[1,"two",null,true]}),

        //13. String with enumeration and pattern not set
        json!({"type":"string","enum":["x","y","z"]}),

        //14. Boolean schema true
        json!(true),

        //15. Boolean schema false (generator may produce invalid – skip generation later)
        json!(false),

        //16. object with additionalProperties schema
        json!({
            "type":"object",
            "properties": {"fixed": {"type":"integer"}},
            "additionalProperties": {"type":"string"}
        }),

        //17. array tuple validation approximated to allOf in AST generator can still satisfy
        json!({"type":"array","items":[{"type":"integer"},{"type":"string"}]}),

        //18. anyOf with object or null
        json!({"anyOf":[{"type":"object"},{"type":"null"}]}),

        //19. nested allOf + anyOf combination
        json!({
            "allOf": [
                {"type":"object","properties":{"a":{"type":"string"}},"required":["a"]},
                {"anyOf":[
                    {"properties":{"b":{"type":"integer"}},"required":["b"]},
                    {"properties":{"c":{"type":"boolean"}},"required":["c"]}
                ]}
            ]
        }),

        //20. array with minItems 0, items = anyOf string|integer
        json!({"type":"array","items":{"anyOf":[{"type":"string"},{"type":"integer"}]}}),
    ]
}

#[test]
fn fixed_corpus_valid_values() {
    let cases = corpus();
    let mut rng = StdRng::seed_from_u64(9999);
    let base = Url::parse("file:///case.json").unwrap();

    for (idx, raw) in cases.into_iter().enumerate() {
        let ast = build_and_resolve_schema(&raw, &base).unwrap();
        let compiled = compile(&ast.to_json()).unwrap();

        // Skip generation for `false` schema (no valid instances)
        if raw == serde_json::Value::Bool(false) {
            assert!(!compiled.is_valid(&json!({})), "false schema must reject");
            continue;
        }

        for _ in 0..50 {
            let v = generate_value(&ast, &mut rng, 5);
            assert!(compiled.is_valid(&v), "schema #{idx} produced invalid value {v}");
        }
    }
}

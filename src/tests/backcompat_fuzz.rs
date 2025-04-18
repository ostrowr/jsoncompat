//! Sample‑based cross‑check that `check_compat` aligns with real validation
//! behaviour for a fixed corpus of schema changes.

use json_schema_backcompat::{build_and_resolve_schema, check_compat, Role};
use json_schema_draft2020::{compile, SchemaNode};
use json_schema_fuzz::generate_value;
use rand::{rngs::StdRng, SeedableRng};
use serde_json::json;
use url::Url;

struct Case {
    old: serde_json::Value,
    new: serde_json::Value,
    expect_ser: bool,
    expect_de: bool,
}

fn cases() -> Vec<Case> {
    vec![
        // 1 Identical schemas
        Case { old: json!({"type":"string"}), new: json!({"type":"string"}), expect_ser: true, expect_de: true },

        // 2 Required → optional
        Case { old: json!({"type":"object","properties":{"id":{"type":"integer"}},"required":["id"]}),
               new: json!({"type":"object","properties":{"id":{"type":"integer"}}}),
               expect_ser:false, expect_de:true },

        // 3 Optional → required
        Case { old: json!({"type":"object","properties":{"id":{"type":"integer"}}}),
               new: json!({"type":"object","properties":{"id":{"type":"integer"}},"required":["id"]}),
               expect_ser:true, expect_de:false },

        // 4 Tighten string minLength
        Case { old: json!({"type":"string","minLength":0}),
               new: json!({"type":"string","minLength":5}),
               expect_ser:true, expect_de:false },

        // 5 Enum expanded
        Case { old: json!({"enum":[1,2]}), new: json!({"enum":[1,2,3]}), expect_ser:false, expect_de:true },

        // 6 integer -> number (broaden)
        Case { old: json!({"type":"integer"}), new: json!({"type":"number"}), expect_ser:false, expect_de:true },

        // 7 tighten additionalProperties
        Case { old: json!({"type":"object"}),
               new: json!({"type":"object","additionalProperties":false}),
               expect_ser:true, expect_de:false },

        // 8 loosen additionalProperties
        Case { old: json!({"type":"object","additionalProperties":false}),
               new: json!({"type":"object"}),
               expect_ser:false, expect_de:true },

        // 9 Numeric minimum increased
        Case { old: json!({"type":"number","minimum":0}),
               new: json!({"type":"number","minimum":10}),
               expect_ser:true, expect_de:false },

        //10 Array maxItems decreased (narrower)
        Case { old: json!({"type":"array","items":{"type":"integer"},"maxItems":10}),
               new: json!({"type":"array","items":{"type":"integer"},"maxItems":5}),
               expect_ser:true, expect_de:false },

        //11 anyOf broadened
        Case { old: json!({"type":"string"}),
               new: json!({"anyOf":[{"type":"string"},{"type":"number"}]}),
               expect_ser:false, expect_de:true },

        //12 allOf tightened (intersect ranges)
        Case { old: json!({"type":"integer","minimum":0,"maximum":100}),
               new: json!({"allOf":[{"type":"integer"},{"minimum":10,"maximum":20}]}),
               expect_ser:true, expect_de:false },

        //13 enum narrowed
        Case { old: json!({"enum":["a","b","c"]}), new: json!({"enum":["a","b"]}), expect_ser:true, expect_de:false },

        //14 type change string -> boolean (incompatible both)
        Case { old: json!({"type":"string"}), new: json!({"type":"boolean"}), expect_ser:false, expect_de:false },

        //15 add new optional property (broadens)
        Case { old: json!({"type":"object","properties":{"a":{"type":"string"}}}),
               new: json!({"type":"object","properties":{"a":{"type":"string"},"b":{"type":"integer"}}}),
               expect_ser:false, expect_de:true },
    ]
}

fn to_ast(v: &serde_json::Value) -> SchemaNode {
    build_and_resolve_schema(v, &Url::parse("file:///s.json").unwrap()).unwrap()
}

#[test]
fn corpus_compat_sampled() {
    let mut rng = StdRng::seed_from_u64(2024);
    for (idx, case) in cases().into_iter().enumerate() {
        let ast_old = to_ast(&case.old);
        let ast_new = to_ast(&case.new);

        // compiled validators
        let comp_old = compile(&ast_old.to_json()).unwrap();
        let comp_new = compile(&ast_new.to_json()).unwrap();

        // ---- serializer role check vs samples ----
        let compat_ser = check_compat(&ast_old, &ast_new, Role::Serializer);
        assert_eq!(compat_ser, case.expect_ser, "case {idx} serializer expectation");

        // sample‑based: generate from NEW, verify OLD accepts
        for _ in 0..50 {
            let v = generate_value(&ast_new, &mut rng, 5);
            if case.expect_ser {
                assert!(comp_old.is_valid(&v), "serializer case {idx} produced counter‑example {v}");
            } else if !comp_old.is_valid(&v) {
                break; // found witness, good
            }
        }

        // ---- deserializer role ----
        let compat_de = check_compat(&ast_old, &ast_new, Role::Deserializer);
        assert_eq!(compat_de, case.expect_de, "case {idx} deserializer expectation");

        for _ in 0..50 {
            let v = generate_value(&ast_old, &mut rng, 5);
            if case.expect_de {
                assert!(comp_new.is_valid(&v), "deserializer case {idx} counter‑example {v}");
            } else if !comp_new.is_valid(&v) {
                break;
            }
        }
    }
}

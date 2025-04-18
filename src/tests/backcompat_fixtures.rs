//! Loads schema‑change pairs from `tests/fixtures/backcompat/*` and asserts
//! our `check_compat` outcome matches the expectation JSON.

use json_schema_backcompat::{build_and_resolve_schema, check_compat, Role};
use json_schema_draft2020::SchemaNode;
use json_schema_draft2020::compile;
use json_schema_fuzz::generate_value;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use url::Url;
use rand::{SeedableRng, rngs::StdRng};

#[derive(Deserialize)]
struct Expectation {
    serializer: bool,
    deserializer: bool,
    #[serde(default)]
    allowed_failure: bool,
}

fn to_ast(v: &serde_json::Value) -> SchemaNode {
    build_and_resolve_schema(v, &Url::parse("file:///fixture.json").unwrap()).unwrap()
}

#[test]
fn fixture_backcompat() {
    let dir = Path::new("tests/fixtures/backcompat");
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if !path.is_dir() { continue; }

        let old_raw: serde_json::Value = serde_json::from_slice(&fs::read(path.join("old.json")).unwrap()).unwrap();
        let new_raw: serde_json::Value = serde_json::from_slice(&fs::read(path.join("new.json")).unwrap()).unwrap();
        let expect: Expectation = serde_json::from_slice(&fs::read(path.join("expect.json")).unwrap()).unwrap();

        let old_ast = to_ast(&old_raw);
        let new_ast = to_ast(&new_raw);

        let ser = check_compat(&old_ast, &new_ast, Role::Serializer);
        let de  = check_compat(&old_ast, &new_ast, Role::Deserializer);

        if expect.allowed_failure {
            // At the moment we *expect* our implementation to be wrong here –
            // test passes when the outcome differs so that CI stays green but
            // the fixture remains visible for future work.
            if ser == expect.serializer && de == expect.deserializer {
                panic!("Previously‑failing fixture now passes – remove \"allowed_failure\" from {:?}", path);
            }
        } else {
            assert_eq!(ser, expect.serializer, "serializer mismatch in {:?}", path);
            assert_eq!(de,  expect.deserializer, "deserializer mismatch in {:?}", path);
        }

        // --- Sample‑based confirmation using fuzz ---
        let mut rng = StdRng::seed_from_u64(0xCAFE + path.to_string_lossy().len() as u64);
        let compiled_old = compile(&old_ast.to_json()).expect("compile old");
        let compiled_new = compile(&new_ast.to_json()).expect("compile new");

        // Check serializer expectation by generating from NEW
        for _ in 0..20 {
            let val = generate_value(&new_ast, &mut rng, 4);
            let old_accepts = compiled_old.is_valid(&val);
            if expect.serializer {
                assert!(old_accepts, "serializer=true but old rejects value {:?} (fixture {:?})", val, path);
            } else if !old_accepts {
                break; // found witness – good
            }
        }

        // Check deserializer expectation by generating from OLD
        for _ in 0..20 {
            let val = generate_value(&old_ast, &mut rng, 4);
            let new_accepts = compiled_new.is_valid(&val);
            if expect.deserializer {
                assert!(new_accepts, "deserializer=true but new rejects value {:?} (fixture {:?})", val, path);
            } else if !new_accepts {
                break;
            }
        }
    }
}

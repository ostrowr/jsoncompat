use json_schema_backcompat::build_and_resolve_schema;
use json_schema_draft2020::compile;
use json_schema_fuzz::generate_value;
use rand::{rngs::StdRng, SeedableRng};
use serde_json::Value;
use std::fs;
use std::path::Path;

// One test per JSON file in fixtures/fuzz.
datatest_stable::harness!(fixture, "tests/fixtures/fuzz", ".*\\.json$");

fn fixture(file: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(file)?;
    let root: Value = serde_json::from_slice(&bytes)?;

    // Extract all schemas contained in the file.  For the canonical JSON‑Schema
    // Test Suite the structure is:
    //     [ { "schema": { .. }, "tests": [ … ] }, … ]
    let mut schemas: Vec<Value> = Vec::new();

    match &root {
        Value::Array(groups) => {
            for item in groups {
                if let Some(s) = item.get("schema") {
                    schemas.push(s.clone());
                }
            }
        }
        _ => panic!(
            "Expected root JSON value to be an array of test groups, got: {:?}",
            root
        ),
    }

    // Deterministic RNG per‑file so the tests are reproducible.
    let seed = 0xBADF00D + file.to_string_lossy().len() as u64;
    let mut rng = StdRng::seed_from_u64(seed);

    for (idx, schema_json) in schemas.iter().enumerate() {
        // `false` schemas have an empty instance set – skip.
        if schema_json == &Value::Bool(false) {
            continue;
        }

        let ast = build_and_resolve_schema(schema_json)?;
        let compiled = compile(schema_json)?;

        // Try a handful of samples; succeed only if **all** validate.
        let mut success = true;
        for _ in 0..256 {
            let v = generate_value(&ast, &mut rng, 6);
            if !compiled.is_valid(&v) {
                eprintln!(
                    "Generated an invalid instance for schema\nSchema:\n{:#}\n\nInvalid instance:\n{:#}\n",
                    schema_json, v
                );
                success = false;
            }
        }

        assert!(
            success,
            "Generated invalid instances for schema #{idx} in {:?}",
            file
        );
    }

    Ok(())
}

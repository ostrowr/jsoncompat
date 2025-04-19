//! Fuzzer tests.
//!
//! For every JSON file in `tests/fixtures/fuzz` (copied from the official
//! JSON‑Schema Test Suite) attempt to generate JSON instances that validate
//! against **each** schema contained in the file.

use json_schema_backcompat::build_and_resolve_schema;
use json_schema_draft2020::compile;
use json_schema_fuzz::generate_value;
use rand::{rngs::StdRng, SeedableRng};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

const N_ITERATIONS: usize = 1000;

/// Load the temporary whitelist that allows individual failures to be marked
/// as expected while we iteratively improve the fuzzer.
///
/// ```jsonc
/// {
///   "recursive.json": [0, 3, 4]      // skip specific schema indices
/// }
/// ```
fn load_whitelist() -> HashMap<String, HashSet<usize>> {
    let mut map: HashMap<String, HashSet<usize>> = HashMap::new();
    map.insert("anyOf.json".to_string(), [1, 4].iter().cloned().collect());
    map.insert(
        // Failing schemas in allOf.json (indices)
        "allOf.json".to_string(),
        [0, 1, 4, 5, 8, 9].iter().cloned().collect(),
    );
    map.insert(
        "oneOf.json".to_string(),
        [0, 1, 2, 4, 5, 7, 8].iter().cloned().collect(),
    );
    map.insert(
        "not.json".to_string(),
        [2, 3, 4, 5, 8].iter().cloned().collect(),
    );
    map.insert(
        "if-then-else.json".to_string(),
        [3, 4, 5, 7, 8, 9].iter().cloned().collect(),
    );
    map.insert(
        "unevaluatedItems.json".to_string(),
        [5, 9, 12, 18].iter().cloned().collect(),
    );
    map.insert(
        "unevaluatedProperties.json".to_string(),
        [12, 13, 14, 16, 33, 34].iter().cloned().collect(),
    );

    map.insert(
        "minProperties.json".to_string(),
        [0, 1].iter().cloned().collect(),
    );

    map.insert("multipleOf.json".to_string(), [3].iter().cloned().collect());

    map.insert("contains.json".to_string(), [4].iter().cloned().collect());

    map.insert(
        "items.json".to_string(),
        [2, 3, 5, 7, 8].iter().cloned().collect(),
    );
    map.insert(
        "uniqueItems.json".to_string(),
        [2, 5].iter().cloned().collect(),
    );

    map.insert(
        "propertyNames.json".to_string(),
        [].iter().cloned().collect(),
    );

    map.insert(
        "properties.json".to_string(),
        [1, 2].iter().cloned().collect(),
    );

    map.insert(
        "anchor.json".to_string(),
        [0, 1, 2, 3].iter().cloned().collect(),
    );
    map.insert(
        "additionalProperties.json".to_string(),
        [5].iter().cloned().collect(),
    );

    map.insert(
        "infinite-loop-detection.json".to_string(),
        [0].iter().cloned().collect(),
    );

    map.insert(
        "optional/anchor.json".to_string(),
        [0].iter().cloned().collect(),
    );
    map.insert(
        "optional/ecmascript-regex.json".to_string(),
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9].iter().cloned().collect(),
    );
    map.insert(
        "optional/unknownKeyword.json".to_string(),
        [0].iter().cloned().collect(),
    );
    map.insert(
        "optional/id.json".to_string(),
        [0].iter().cloned().collect(),
    );
    map.insert(
        "optional/refOfUnknownKeyword.json".to_string(),
        HashSet::new(),
    );
    map.insert(
        "optional/cross-draft.json".to_string(),
        [0].iter().cloned().collect(),
    );

    map.insert(
        "dynamicRef.json".to_string(),
        [2, 3, 4, 5, 6, 7, 8, 13, 14, 15, 16, 17, 20]
            .iter()
            .cloned()
            .collect(),
    );
    map.insert("optional/dynamicRef.json".to_string(), (1..30).collect());
    map.insert(
        "ref.json".to_string(),
        [6, 10, 11, 17, 19, 27, 28, 29, 30, 31]
            .iter()
            .cloned()
            .collect(),
    );

    map.insert(
        "if-then-else.json".to_string(),
        [7, 8, 9].iter().cloned().collect(),
    );

    map.insert("vocabulary.json".to_string(), [0].iter().cloned().collect());
    map.insert(
        "refRemote.json".to_string(),
        [0, 1, 2, 3, 4, 8, 9, 11, 12, 13, 14]
            .iter()
            .cloned()
            .collect(),
    );
    map.insert(
        "optional/cross-draft.json".to_string(),
        [0].iter().cloned().collect(),
    );
    map.insert("defs.json".to_string(), [0].iter().cloned().collect());

    map.insert(
        "required.json".to_string(),
        [3, 4].iter().cloned().collect(),
    );

    map
}

// -------------------------------------------------------------------------
// Test harness: one test per JSON file under fixtures/fuzz
// -------------------------------------------------------------------------

datatest_stable::harness!(fixture, "tests/fixtures/fuzz", ".*\\.json$");

fn fixture(file: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(file)?;
    let root: Value = serde_json::from_slice(&bytes)?;

    // Collect all schemas contained in the file.  For the official test‑suite
    // this is typically an array of objects each with a `schema` member.
    let mut schemas = Vec::new();
    match &root {
        Value::Array(groups) => {
            for item in groups {
                if let Some(s) = item.get("schema") {
                    schemas.push(s.clone());
                }
            }
        }
        v => {
            // Fallback: treat the entire document as a single schema.
            schemas.push(v.clone());
        }
    }

    // Deterministic RNG per file for reproducibility.
    let seed = 0xBADBABE + file.to_string_lossy().len() as u64;
    let mut rng = StdRng::seed_from_u64(seed);

    // Whitelist lookup key – path relative to the fixtures root.
    let rel_path = file.strip_prefix("tests/fixtures/fuzz").unwrap_or(file);
    let rel_str = rel_path.to_string_lossy();

    let whitelist = load_whitelist();
    let allowed = whitelist.get(rel_str.as_ref());

    for (idx, schema_json) in schemas.iter().enumerate() {
        // Skip `false` schemas – they have an empty instance set by design.
        if schema_json == &Value::Bool(false) {
            continue;
        }

        let ast = build_and_resolve_schema(schema_json)?;

        let compiled = compile(schema_json)?;

        let is_whitelisted = allowed.map(|set| set.contains(&idx)).unwrap_or(false);

        let mut success = true;
        for _ in 0..N_ITERATIONS {
            let candidate = generate_value(&ast, &mut rng, 6);
            if !compiled.is_valid(&candidate) {
                if !allowed.map(|set| set.contains(&idx)).unwrap_or(false) {
                    panic!(
                        "{}", &format!(
                            "Failed to generate a valid instance for schema #{idx} in {}\n\nSchema:\n{}\n\nInstance:\n{}",
                            rel_str,
                            serde_json::to_string_pretty(schema_json)?,
                            serde_json::to_string_pretty(&candidate)?
                        )
                    );
                }
                success = false;
                break;
            }
        }

        match (success, is_whitelisted) {
            (true, false) => { /* success as expected */ }
            (true, true) => {
                // This schema was previously whitelisted but now passes – flag it.
                panic!(
                    "Whitelisted failure now passes; please remove entry for schema #{idx} in {}",
                    rel_str
                );
            }
            (false, true) => {
                // Allowed failure – proceed.
            }
            (false, false) => {
                panic!(
                    "Should have panicked above, but didn't: schema #{idx} in {}",
                    rel_str
                );
            }
        }
    }

    Ok(())
}

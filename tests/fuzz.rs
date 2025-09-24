//! Fuzzer tests.

use json_schema_ast::compile;
use json_schema_fuzz::{generate_value, GenerateError};
use jsoncompat::build_and_resolve_schema;
use rand::{rngs::StdRng, SeedableRng};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

const N_ITERATIONS: usize = 1000;

#[derive(Debug)]
enum GenerationOutcome {
    Valid,
    Invalid(Value),
    Unsatisfiable,
    Exhausted,
}

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
    map.insert("anyOf.json".to_string(), [4].iter().cloned().collect());
    map.insert("oneOf.json".to_string(), [2, 4].iter().cloned().collect());
    map.insert("not.json".to_string(), [2, 8].iter().cloned().collect());
    map.insert(
        "if-then-else.json".to_string(),
        [3, 4, 5, 7, 8, 9].iter().cloned().collect(),
    );
    map.insert(
        "unevaluatedItems.json".to_string(),
        [9, 12, 18].iter().cloned().collect(),
    );
    map.insert(
        "unevaluatedProperties.json".to_string(),
        [12, 13, 14, 16, 33].iter().cloned().collect(),
    );

    map.insert(
        "items.json".to_string(),
        [2, 5, 7, 8].iter().cloned().collect(),
    );
    map.insert(
        "uniqueItems.json".to_string(),
        [2, 5].iter().cloned().collect(),
    );

    map.insert(
        "anchor.json".to_string(),
        [0, 1, 2, 3].iter().cloned().collect(),
    );
    map.insert(
        "optional/ecmascript-regex.json".to_string(),
        [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
            31,
        ]
        .iter()
        .cloned()
        .collect(),
    );
    map.insert(
        "optional/non-bmp-regex.json".to_string(),
        [0].iter().cloned().collect(),
    );
    map.insert("pattern.json".to_string(), [0, 1].iter().cloned().collect());
    map.insert(
        "optional/id.json".to_string(),
        [0].iter().cloned().collect(),
    );
    map.insert(
        "optional/cross-draft.json".to_string(),
        [0].iter().cloned().collect(),
    );

    map.insert(
        "dynamicRef.json".to_string(),
        [2, 13, 14, 15, 16, 17, 20].iter().cloned().collect(),
    );
    map.insert("optional/dynamicRef.json".to_string(), (1..30).collect());
    map.insert(
        "ref.json".to_string(),
        [6, 10, 19, 26].iter().cloned().collect(),
    );

    map.insert(
        "if-then-else.json".to_string(),
        [7, 8].iter().cloned().collect(),
    );

    map.insert(
        "refRemote.json".to_string(),
        [0, 1, 2, 3, 4, 5, 6, 8, 9, 11, 12, 13, 14]
            .iter()
            .cloned()
            .collect(),
    );
    map.insert(
        "optional/cross-draft.json".to_string(),
        [0].iter().cloned().collect(),
    );
    map.insert("defs.json".to_string(), [0].iter().cloned().collect());

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
                    let expect_unsat = item
                        .get("jsoncompat_expect_unsat")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    schemas.push((s.clone(), expect_unsat));
                }
            }
        }
        v => {
            // Fallback: treat the entire document as a single schema.
            schemas.push((v.clone(), false));
        }
    }

    // Deterministic RNG per file for reproducibility.
    let seed = 0xBADBABE + file.to_string_lossy().len() as u64;
    let mut rng = StdRng::seed_from_u64(seed);

    // Whitelist lookup key – path relative to the fixtures root.
    let rel_path = file.strip_prefix("tests/fixtures/fuzz").unwrap_or(file);
    let rel_str = rel_path.to_string_lossy().replace('\\', "/");

    let whitelist = load_whitelist();
    let allowed = whitelist.get::<str>(rel_str.as_ref());

    for (idx, (schema_json, expect_unsat)) in schemas.iter().enumerate() {
        let ast = build_and_resolve_schema(schema_json)?;

        let compiled = compile(schema_json)?;

        let is_whitelisted = allowed.map(|set| set.contains(&idx)).unwrap_or(false);

        let mut outcome = GenerationOutcome::Exhausted;
        for _ in 0..N_ITERATIONS {
            match generate_value(&ast, &mut rng, 6) {
                Ok(candidate) => {
                    if compiled.is_valid(&candidate) {
                        outcome = GenerationOutcome::Valid;
                        break;
                    } else {
                        outcome = GenerationOutcome::Invalid(candidate);
                        break;
                    }
                }
                Err(GenerateError::Unsatisfiable) => {
                    outcome = GenerationOutcome::Unsatisfiable;
                    break;
                }
                Err(GenerateError::Exhausted) => {
                    outcome = GenerationOutcome::Exhausted;
                }
            }
        }

        match outcome {
            GenerationOutcome::Valid => {
                if is_whitelisted {
                    panic!(
                        "Whitelisted failure now passes; please remove entry for schema #{idx} in {rel_str}"
                    );
                }
            }
            GenerationOutcome::Invalid(candidate) => {
                if is_whitelisted {
                    continue;
                }
                panic!(
                    "{}",
                    &format!(
                        "Failed to generate a valid instance for schema #{idx} in {}\n\nSchema:\n{}\n\nInstance:\n{}",
                        rel_str,
                        serde_json::to_string_pretty(schema_json)?,
                        serde_json::to_string_pretty(&candidate)?
                    )
                );
            }
            GenerationOutcome::Exhausted => {
                // TODO: a different expect_unsat could be allowed here
                // for non provably unsatisfiable schemas
                if is_whitelisted {
                    continue;
                }
                panic!(
                    "{}",
                    &format!(
                        "Generator exhausted without finding a value for schema #{idx} in {}\n\nSchema:\n{}",
                        rel_str,
                        serde_json::to_string_pretty(schema_json)?
                    )
                );
            }
            GenerationOutcome::Unsatisfiable => {
                if *expect_unsat {
                    continue;
                }
                if is_whitelisted {
                    continue;
                }
                panic!(
                    "{}",
                    &format!(
                        "Generator determined schema #{idx} in {} is unsatisfiable\n\nSchema:\n{}",
                        rel_str,
                        serde_json::to_string_pretty(schema_json)?
                    )
                );
            }
        }
    }

    Ok(())
}

//! Fuzzer integration tests.
//!
//! For every JSON file in `tests/fixtures/fuzz` (copied from the official
//! JSON‑Schema Test Suite) we attempt to generate at least *one* JSON instance
//! that validates against **every** schema contained in the file.  The
//! generation is performed by `json_schema_fuzz::generate_value` and the
//! candidate instance is checked with the authoritative Draft 2020‑12
//! validator from `json_schema_draft2020`.

use json_schema_backcompat::build_and_resolve_schema;
use json_schema_draft2020::compile;
use json_schema_fuzz::generate_value;
use rand::{rngs::StdRng, SeedableRng};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

// -------------------------------------------------------------------------
// Whitelist handling
// -------------------------------------------------------------------------

/// Load the *optional* whitelist that allows individual failures to be marked
/// as expected while we iteratively improve the fuzzer.  The file lives at
/// `tests/fixtures/fuzz/whitelist.json` and has the following shape:
///
/// ```jsonc
/// {
///   "recursive.json": [0, 3, 4]      // skip specific schema indices
/// }
/// ```
fn load_whitelist() -> HashMap<String, HashSet<usize>> {
    let path = Path::new("tests/fixtures/fuzz/whitelist.json");
    let raw = match fs::read(path) {
        Ok(b) => b,
        Err(_) => return HashMap::new(),
    };

    let doc: Value = match serde_json::from_slice(&raw) {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };

    let mut map = HashMap::new();
    if let Value::Object(obj) = doc {
        for (k, v) in obj {
            let set: HashSet<usize> = match v {
                Value::Array(arr) => arr
                    .into_iter()
                    .filter_map(|vv| vv.as_u64().map(|n| n as usize))
                    .collect(),
                _ => panic!(
                    "whitelist.json: expected array of indices for {}, got {}",
                    k, v
                ),
            };
            map.insert(k, set);
        }
    }
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
    let seed = 0xF00DBABE + file.to_string_lossy().len() as u64;
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

        let mut success = false;
        for _ in 0..256 {
            let candidate = generate_value(&ast, &mut rng, 6);
            if compiled.is_valid(&candidate) {
                success = true;
                break;
            }
        }

        // Determine if this particular (file, schema‑index) is whitelisted.
        let is_whitelisted = allowed.map(|set| set.contains(&idx)).unwrap_or(false);

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
                    "Could not generate a valid instance for schema #{idx} in {}",
                    rel_str
                );
            }
        }
    }

    Ok(())
}

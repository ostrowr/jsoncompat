//! Fuzzer tests.

use json_schema_ast::{AstError, SchemaError, compile};
use json_schema_fuzz::ValueGenerator;
use jsoncompat::{SchemaNodeKind, build_and_resolve_schema};
use rand::{SeedableRng, rngs::StdRng};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

const N_ITERATIONS: usize = 1000;
const JSON_SCHEMA_DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";
const JSON_SCHEMA_DRAFT_2020_12_WITH_FRAGMENT: &str =
    "https://json-schema.org/draft/2020-12/schema#";

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
    map.insert("not.json".to_string(), [8].iter().cloned().collect());
    map.insert(
        "unevaluatedItems.json".to_string(),
        [12, 18].iter().cloned().collect(),
    );
    map.insert(
        "unevaluatedProperties.json".to_string(),
        [12, 16].iter().cloned().collect(),
    );

    map.insert(
        "anchor.json".to_string(),
        [0, 1, 2, 3].iter().cloned().collect(),
    );
    map.insert(
        "optional/anchor.json".to_string(),
        [0].iter().cloned().collect(),
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
        "optional/cross-draft.json".to_string(),
        [0].iter().cloned().collect(),
    );

    map.insert(
        "dynamicRef.json".to_string(),
        [3, 4, 5, 6, 7, 8, 13, 14, 15, 16, 17]
            .iter()
            .cloned()
            .collect(),
    );
    map.insert(
        "ref.json".to_string(),
        [6, 10, 17, 19, 27, 28, 29, 30, 31]
            .iter()
            .cloned()
            .collect(),
    );

    map.insert(
        "refRemote.json".to_string(),
        [0, 1, 2, 3, 8, 9, 11, 12, 13, 14].iter().cloned().collect(),
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
    let schemas = collect_fixture_schemas(&root);

    // Deterministic RNG per file for reproducibility.
    let seed = 0xBADBABE + file.to_string_lossy().len() as u64;
    let mut rng = StdRng::seed_from_u64(seed);

    // Whitelist lookup key – path relative to the fixtures root.
    let rel_path = file.strip_prefix("tests/fixtures/fuzz").unwrap_or(file);
    let rel_str = rel_path.to_string_lossy().replace('\\', "/");
    let validate_fixture_tests = rel_str.starts_with("custom/");

    let whitelist = load_whitelist();
    let allowed = whitelist.get::<str>(rel_str.as_ref());

    for (idx, fixture_schema) in schemas.iter().enumerate() {
        let schema_json = &fixture_schema.schema;
        // Skip `false` schemas – they have an empty instance set by design.
        if schema_json == &Value::Bool(false) {
            continue;
        }

        let ast = match build_and_resolve_schema(schema_json) {
            Ok(ast) => ast,
            Err(error)
                if !validate_fixture_tests
                    && schema_declares_unsupported_schema_uri(schema_json) =>
            {
                assert!(
                    matches!(
                        error,
                        AstError::Schema(
                            SchemaError::UnsupportedSchemaDialect {
                                ref pointer,
                                expected_uri: JSON_SCHEMA_DRAFT_2020_12,
                                ..
                            }
                        ) if pointer == "#/$schema"
                    ),
                    "unexpected unsupported-$schema error for {rel_str} schema #{idx}: {error}"
                );
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        if matches!(ast.kind(), SchemaNodeKind::BoolSchema(false))
            && !fixture_schema
                .tests
                .iter()
                .any(|fixture_test| fixture_test.valid)
        {
            continue;
        }

        let compiled = compile(schema_json)?;
        let mut generator = ValueGenerator::new();

        if validate_fixture_tests {
            for fixture_test in &fixture_schema.tests {
                assert_eq!(
                    compiled.is_valid(&fixture_test.data),
                    fixture_test.valid,
                    "{} schema #{idx} ({}) fixture test {:?} returned the wrong validation result\n\nSchema:\n{}\n\nInstance:\n{}",
                    rel_str,
                    fixture_schema.description,
                    fixture_test.description,
                    serde_json::to_string_pretty(schema_json)?,
                    serde_json::to_string_pretty(&fixture_test.data)?,
                );
            }
        }

        let is_whitelisted = allowed.map(|set| set.contains(&idx)).unwrap_or(false);

        let mut success = true;
        for _ in 0..N_ITERATIONS {
            let candidate = generator.generate_value(&ast, &mut rng, 6);
            if !compiled.is_valid(&candidate) {
                if !allowed.map(|set| set.contains(&idx)).unwrap_or(false) {
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
                success = false;
                break;
            }
        }

        match (success, is_whitelisted) {
            (true, false) => { /* success as expected */ }
            (true, true) => {
                // This schema was previously whitelisted but now passes – flag it.
                panic!(
                    "Whitelisted failure now passes; please remove entry for schema #{idx} in {rel_str}"
                );
            }
            (false, true) => {
                // Allowed failure – proceed.
            }
            (false, false) => {
                panic!("Should have panicked above, but didn't: schema #{idx} in {rel_str}");
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
struct FixtureSchema {
    description: String,
    schema: Value,
    tests: Vec<FixtureTest>,
}

#[derive(Debug)]
struct FixtureTest {
    description: String,
    data: Value,
    valid: bool,
}

fn collect_fixture_schemas(root: &Value) -> Vec<FixtureSchema> {
    match root {
        Value::Array(groups) => groups
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let schema = item.get("schema")?.clone();
                let description = item
                    .get("description")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("schema #{index}"));
                let tests = item
                    .get("tests")
                    .and_then(Value::as_array)
                    .map(|tests| {
                        tests
                            .iter()
                            .enumerate()
                            .filter_map(|(test_index, test)| {
                                Some(FixtureTest {
                                    description: test
                                        .get("description")
                                        .and_then(Value::as_str)
                                        .map(str::to_owned)
                                        .unwrap_or_else(|| format!("test #{test_index}")),
                                    data: test.get("data")?.clone(),
                                    valid: test.get("valid")?.as_bool()?,
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Some(FixtureSchema {
                    description,
                    schema,
                    tests,
                })
            })
            .collect(),
        schema => vec![FixtureSchema {
            description: "root schema".to_owned(),
            schema: schema.clone(),
            tests: Vec::new(),
        }],
    }
}

fn schema_declares_unsupported_schema_uri(schema: &Value) -> bool {
    match schema {
        Value::Object(object) => {
            if let Some(schema_uri) = object.get("$schema")
                && !matches!(
                    schema_uri.as_str(),
                    Some(JSON_SCHEMA_DRAFT_2020_12 | JSON_SCHEMA_DRAFT_2020_12_WITH_FRAGMENT)
                )
            {
                return true;
            }
            object.values().any(schema_declares_unsupported_schema_uri)
        }
        Value::Array(items) => items.iter().any(schema_declares_unsupported_schema_uri),
        _ => false,
    }
}

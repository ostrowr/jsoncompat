//! Fuzzer tests.

use json_schema_ast::{
    AstError, NodeId, PatternSupport, SchemaDocument, SchemaError, SchemaNode, SchemaNodeKind,
    compile,
};
use json_schema_fuzz::{GenerateError, GenerationConfig, ValueGenerator};
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
        [12].iter().cloned().collect(),
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

        let is_whitelisted = allowed.map(|set| set.contains(&idx)).unwrap_or(false);

        let schema = match SchemaDocument::from_json(schema_json) {
            Ok(schema) => schema,
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
            Err(
                error @ (AstError::UnsupportedReference { .. }
                | AstError::UnresolvedReference { .. }),
            ) if !validate_fixture_tests => {
                assert!(
                    !validate_fixture_tests,
                    "schema #{idx} in {rel_str} failed with an expected resolver error: {error}"
                );
                continue;
            }
            Err(error) => return Err(error.into()),
        };

        let root = match schema.root() {
            Ok(root) => root,
            Err(
                error @ (AstError::UnsupportedReference { .. }
                | AstError::UnresolvedReference { .. }),
            ) if !validate_fixture_tests => {
                assert!(
                    !validate_fixture_tests,
                    "schema #{idx} in {rel_str} failed with an expected resolver error: {error}"
                );
                continue;
            }
            Err(error) => return Err(error.into()),
        };

        if matches!(root.kind(), SchemaNodeKind::BoolSchema(false))
            && !fixture_schema
                .tests
                .iter()
                .any(|fixture_test| fixture_test.valid)
        {
            continue;
        }

        let generation_config = GenerationConfig::new(6);
        let evaluator_should_be_exact =
            internal_evaluator_should_be_exact(root, schema.canonical_schema_json()?);
        let canonical_validator = compile(schema.canonical_schema_json()?)?;

        if validate_fixture_tests {
            for fixture_test in &fixture_schema.tests {
                let raw_valid = schema.is_valid(&fixture_test.data)?;
                let canonicalized_valid = canonical_validator.is_valid(&fixture_test.data);
                assert_eq!(
                    raw_valid,
                    canonicalized_valid,
                    "{} schema #{idx} ({}) fixture test {:?} validates differently after canonicalization\n\nRaw schema:\n{}\n\nCanonicalized schema:\n{}\n\nInstance:\n{}",
                    rel_str,
                    fixture_schema.description,
                    fixture_test.description,
                    serde_json::to_string_pretty(schema_json)?,
                    serde_json::to_string_pretty(schema.canonical_schema_json()?)?,
                    serde_json::to_string_pretty(&fixture_test.data)?,
                );
                assert_eq!(
                    raw_valid,
                    fixture_test.valid,
                    "{} schema #{idx} ({}) fixture test {:?} returned the wrong validation result\n\nSchema:\n{}\n\nInstance:\n{}",
                    rel_str,
                    fixture_schema.description,
                    fixture_test.description,
                    serde_json::to_string_pretty(schema_json)?,
                    serde_json::to_string_pretty(&fixture_test.data)?,
                );
                if evaluator_should_be_exact {
                    assert_eq!(
                        root.accepts_value(&fixture_test.data),
                        canonicalized_valid,
                        "{} schema #{idx} ({}) fixture test {:?} is handled differently by the internal evaluator\n\nCanonicalized schema:\n{}\n\nInstance:\n{}",
                        rel_str,
                        fixture_schema.description,
                        fixture_test.description,
                        serde_json::to_string_pretty(schema.canonical_schema_json()?)?,
                        serde_json::to_string_pretty(&fixture_test.data)?,
                    );
                }
            }
        }

        let mut success = true;
        for _ in 0..N_ITERATIONS {
            let candidate = match ValueGenerator::generate(&schema, generation_config, &mut rng) {
                Ok(candidate) => candidate,
                Err(GenerateError::ExhaustedAttempts { .. }) => {
                    if !allowed.map(|set| set.contains(&idx)).unwrap_or(false) {
                        panic!(
                            "{}",
                            &format!(
                                "Failed to generate a valid instance for schema #{idx} in {}\n\nSchema:\n{}",
                                rel_str,
                                serde_json::to_string_pretty(schema_json)?,
                            )
                        );
                    }
                    success = false;
                    break;
                }
                Err(GenerateError::Schema(error)) => return Err(error.into()),
                Err(error) => return Err(error.into()),
            };

            let raw_valid = schema.is_valid(&candidate)?;
            let canonicalized_valid = canonical_validator.is_valid(&candidate);
            assert_eq!(
                raw_valid,
                canonicalized_valid,
                "{} schema #{idx} generated candidate validates differently after canonicalization\n\nRaw schema:\n{}\n\nCanonicalized schema:\n{}\n\nInstance:\n{}",
                rel_str,
                serde_json::to_string_pretty(schema_json)?,
                serde_json::to_string_pretty(schema.canonical_schema_json()?)?,
                serde_json::to_string_pretty(&candidate)?,
            );
            assert!(
                raw_valid,
                "generator returned a value rejected by the raw schema for {rel_str} schema #{idx}: {}",
                serde_json::to_string_pretty(&candidate)?,
            );
            if evaluator_should_be_exact {
                assert_eq!(
                    root.accepts_value(&candidate),
                    canonicalized_valid,
                    "{rel_str} schema #{idx} generated candidate is handled differently by the internal evaluator\n\nCanonicalized schema:\n{}\n\nInstance:\n{}",
                    serde_json::to_string_pretty(schema.canonical_schema_json()?)?,
                    serde_json::to_string_pretty(&candidate)?,
                );
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

fn internal_evaluator_should_be_exact(schema: &SchemaNode, canonical_schema: &Value) -> bool {
    canonical_keywords_are_supported_by_internal_evaluator(canonical_schema)
        && schema_is_supported_by_internal_evaluator(schema, &mut HashSet::new())
}

fn canonical_keywords_are_supported_by_internal_evaluator(schema: &Value) -> bool {
    let Value::Object(object) = schema else {
        return true;
    };

    if object.contains_key("unevaluatedItems")
        || object.contains_key("unevaluatedProperties")
        || object.contains_key("dependentSchemas")
    {
        return false;
    }

    object.values().all(|value| match value {
        Value::Array(values) => values
            .iter()
            .all(canonical_keywords_are_supported_by_internal_evaluator),
        Value::Object(_) => canonical_keywords_are_supported_by_internal_evaluator(value),
        _ => true,
    })
}

fn schema_is_supported_by_internal_evaluator(
    schema: &SchemaNode,
    seen: &mut HashSet<NodeId>,
) -> bool {
    if !seen.insert(schema.id()) {
        return true;
    }

    use SchemaNodeKind::*;

    match schema.kind() {
        BoolSchema(_) | Any | Boolean { .. } | Null { .. } | Const(_) | Enum(_) => true,
        String {
            pattern, format, ..
        } => {
            format.is_none()
                && pattern
                    .as_ref()
                    .is_none_or(|pattern| pattern.support() == PatternSupport::Supported)
        }
        Number { .. } | Integer { .. } => true,
        Object {
            properties,
            pattern_properties,
            additional,
            property_names,
            ..
        } => {
            properties
                .values()
                .all(|schema| schema_is_supported_by_internal_evaluator(schema, seen))
                && pattern_properties.values().all(|pattern_property| {
                    pattern_property.pattern.support() == PatternSupport::Supported
                        && schema_is_supported_by_internal_evaluator(&pattern_property.schema, seen)
                })
                && schema_is_supported_by_internal_evaluator(additional, seen)
                && schema_is_supported_by_internal_evaluator(property_names, seen)
        }
        Array {
            prefix_items,
            items,
            contains,
            ..
        } => {
            prefix_items
                .iter()
                .all(|schema| schema_is_supported_by_internal_evaluator(schema, seen))
                && schema_is_supported_by_internal_evaluator(items, seen)
                && contains.as_ref().is_none_or(|contains| {
                    schema_is_supported_by_internal_evaluator(&contains.schema, seen)
                })
        }
        AllOf(children) | AnyOf(children) | OneOf(children) => children
            .iter()
            .all(|schema| schema_is_supported_by_internal_evaluator(schema, seen)),
        Not(schema) => schema_is_supported_by_internal_evaluator(schema, seen),
        IfThenElse {
            if_schema,
            then_schema,
            else_schema,
        } => {
            schema_is_supported_by_internal_evaluator(if_schema, seen)
                && then_schema
                    .as_ref()
                    .is_none_or(|schema| schema_is_supported_by_internal_evaluator(schema, seen))
                && else_schema
                    .as_ref()
                    .is_none_or(|schema| schema_is_supported_by_internal_evaluator(schema, seen))
        }
        _ => false,
    }
}

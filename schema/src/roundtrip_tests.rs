use crate::build_and_resolve_schema;
use crate::canonicalize::{CanonicalizeError, canonicalize_schema};
use serde_json::Value;
use std::fs;
use std::path::Path;

const JSON_SCHEMA_DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";
const JSON_SCHEMA_DRAFT_2020_12_WITH_FRAGMENT: &str =
    "https://json-schema.org/draft/2020-12/schema#";

#[test]
fn fuzz_fixtures_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let dir = Path::new("../tests/fixtures/fuzz");
    let mut files = Vec::new();
    collect_fixture_files(dir, &mut files)?;
    files.sort();

    for path in files {
        let bytes = fs::read(&path)?;
        let root: Value = serde_json::from_slice(&bytes)?;

        let mut schemas = Vec::new();
        match &root {
            Value::Array(groups) => {
                for item in groups {
                    if let Some(s) = item.get("schema") {
                        schemas.push(s.clone());
                    }
                }
            }
            v => schemas.push(v.clone()),
        }

        for schema_json in schemas {
            if schema_json == Value::Bool(false) {
                continue;
            }

            if schema_declares_unsupported_schema_uri(&schema_json) {
                let error = canonicalize_schema(&schema_json).unwrap_err();
                assert!(matches!(
                    error,
                    CanonicalizeError::UnsupportedSchemaDialect {
                        pointer,
                        expected_uri: JSON_SCHEMA_DRAFT_2020_12,
                        ..
                    } if pointer == "#/$schema"
                ));
                continue;
            }

            let schema = canonicalize_schema(&schema_json)
                .map_err(|error| format!("{} canonicalize: {error}", path.display()))?;
            let ast = build_and_resolve_schema(schema.as_value())
                .map_err(|error| format!("{}: {error}", path.display()))?;
            if ast.has_cycle() {
                // `SchemaNode::to_json()` serializes trees, not graph backedges.
                // Recursive fixtures are still covered by `tests/fuzz.rs`.
                continue;
            }
            let json = ast.to_json();
            let schema2 = canonicalize_schema(&json)
                .map_err(|error| format!("{} roundtrip canonicalize: {error}", path.display()))?;
            let ast2 = build_and_resolve_schema(schema2.as_value())
                .map_err(|error| format!("{} roundtrip: {error}", path.display()))?;
            if ast != ast2 {
                panic!(
                    "roundtrip failed for {}\noriginal: {}\ninput: {}\nroundtrip: {}",
                    path.display(),
                    serde_json::to_string_pretty(&schema_json)?,
                    serde_json::to_string_pretty(&json)?,
                    serde_json::to_string_pretty(&ast2.to_json())?,
                );
            }
        }
    }
    Ok(())
}

fn collect_fixture_files(
    dir: &Path,
    files: &mut Vec<std::path::PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_fixture_files(&path, files)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("json") {
            files.push(path);
        }
    }
    Ok(())
}

fn schema_declares_unsupported_schema_uri(schema: &Value) -> bool {
    let Some(uri) = schema
        .as_object()
        .and_then(|object| object.get("$schema"))
        .and_then(Value::as_str)
    else {
        return false;
    };

    !matches!(
        uri,
        JSON_SCHEMA_DRAFT_2020_12 | JSON_SCHEMA_DRAFT_2020_12_WITH_FRAGMENT
    )
}

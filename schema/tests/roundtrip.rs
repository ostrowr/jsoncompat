use json_schema_ast::build_and_resolve_schema;
use serde_json::Value;
use std::fs;
use std::path::Path;

#[test]
fn fuzz_fixtures_roundtrip_and_ast_stringified_equality() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = Path::new("../tests/fixtures/fuzz");
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
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
            let ast = build_and_resolve_schema(&schema_json)?;
            let json = ast.to_json();
            let ast2 = build_and_resolve_schema(&json)?;

            // Roundtrip equality
            assert_eq!(
                ast.to_json(),
                ast2.to_json(),
                "roundtrip failed for {}",
                path.display()
            );

            // AST to_json stringified equality
            let stringified1 = serde_json::to_string(&ast.to_json())?;
            let stringified2 = serde_json::to_string(&ast2.to_json())?;
            // let original_stringified = serde_json::to_string(&schema_json)?;

            // assert_eq!(original_stringified, stringified1, "original stringified differs from first round stringified for {}", path.display());
            assert_eq!(
                stringified1,
                stringified2,
                "stringified ASTs differ for {}",
                path.display()
            );
        }
    }
    Ok(())
}

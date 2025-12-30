use json_schema_ast::build_and_resolve_schema;
use json_schema_codegen::{pydantic, ModelRole, PydanticOptions};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

#[test]
fn fuzz_fixtures_pydantic_goldens() -> Result<(), Box<dyn std::error::Error>> {
    let regen = std::env::var_os("REGEN_CODEGEN_GOLDENS").is_some();
    let whitelist = build_whitelist();
    let base_module = "json_schema_codegen_base";
    let base_code = pydantic::base_module();

    let fixtures_root = PathBuf::from("../tests/fixtures/fuzz");
    let golden_root = PathBuf::from("tests/golden/pydantic_fuzz");

    if regen && golden_root.exists() {
        for entry in fs::read_dir(&golden_root)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str == "tests"
                || name_str == "pyproject.toml"
                || name_str == "README.md"
                || name_str == "uv.lock"
            {
                continue;
            }
            if path.is_dir() {
                fs::remove_dir_all(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
        }
    }

    let base_path = golden_root.join(format!("{base_module}.py"));
    if regen {
        if let Some(parent) = base_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&base_path, &base_code)?;
    } else {
        let expected_base = fs::read_to_string(&base_path).map_err(|_| {
            format!(
                "Missing base module golden {}; set REGEN_CODEGEN_GOLDENS=1 to refresh",
                base_path.display()
            )
        })?;
        assert_eq!(
            base_code, expected_base,
            "Base module golden mismatch; set REGEN_CODEGEN_GOLDENS=1 to refresh"
        );
    }

    for entry in walkdir::WalkDir::new(&fixtures_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        if entry.path().extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let rel_path = entry
            .path()
            .strip_prefix(&fixtures_root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let content = fs::read(entry.path())?;
        let root: Value = serde_json::from_slice(&content)?;
        let schemas = collect_schemas(&root);

        for (schema_json, idx) in schemas {
            if is_whitelisted(&whitelist, &rel_path, idx) {
                continue;
            }
            if schema_json == Value::Bool(false) {
                continue;
            }

            let schema = build_and_resolve_schema(&schema_json)
                .map_err(|e| format!("{rel_path}#{idx}: {e}"))?;
            let root_name = format!(
                "{}{}",
                sanitize_type_name(entry.path().file_stem().unwrap().to_string_lossy().as_ref()),
                idx
            );
            let allow_non_object_inputs = allows_non_object_inputs(&schema_json);
            let options = PydanticOptions::default()
                .with_root_model_name(root_name.clone())
                .with_base_module(base_module)
                .with_allow_non_object_inputs(allow_non_object_inputs);
            let serializer =
                match pydantic::generate_model(&schema, ModelRole::Serializer, options.clone()) {
                    Ok(code) => code,
                    Err(json_schema_codegen::CodegenError::RootNotObject { .. }) => continue,
                    Err(err) => return Err(format!("{rel_path}#{idx} serializer: {err}").into()),
                };
            let deserializer =
                match pydantic::generate_model(&schema, ModelRole::Deserializer, options) {
                    Ok(code) => code,
                    Err(json_schema_codegen::CodegenError::RootNotObject { .. }) => continue,
                    Err(err) => return Err(format!("{rel_path}#{idx} deserializer: {err}").into()),
                };

            let base_dir = golden_root.join(&rel_path);
            fs::create_dir_all(&base_dir)?;
            let serializer_path = base_dir.join(format!("{idx}_serializer.py"));
            let deserializer_path = base_dir.join(format!("{idx}_deserializer.py"));

            if regen {
                fs::write(&serializer_path, serializer)?;
                fs::write(&deserializer_path, deserializer)?;
                continue;
            }

            let expected_serializer = fs::read_to_string(&serializer_path).map_err(|_| {
                format!(
                    "Missing golden file {}; set REGEN_CODEGEN_GOLDENS=1 to refresh",
                    serializer_path.display()
                )
            })?;
            let expected_deserializer = fs::read_to_string(&deserializer_path).map_err(|_| {
                format!(
                    "Missing golden file {}; set REGEN_CODEGEN_GOLDENS=1 to refresh",
                    deserializer_path.display()
                )
            })?;

            assert_eq!(
                serializer, expected_serializer,
                "Serializer golden mismatch for {rel_path}#{idx}; set REGEN_CODEGEN_GOLDENS=1 to refresh"
            );
            assert_eq!(
                deserializer, expected_deserializer,
                "Deserializer golden mismatch for {rel_path}#{idx}; set REGEN_CODEGEN_GOLDENS=1 to refresh"
            );
        }
    }

    Ok(())
}

fn collect_schemas(root: &Value) -> Vec<(Value, usize)> {
    match root {
        Value::Array(groups) => {
            let mut out = Vec::new();
            for (idx, item) in groups.iter().enumerate() {
                if let Some(schema) = item.get("schema") {
                    out.push((schema.clone(), idx));
                }
            }
            if out.is_empty() {
                vec![(root.clone(), 0)]
            } else {
                out
            }
        }
        _ => vec![(root.clone(), 0)],
    }
}

fn sanitize_type_name(input: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            if capitalize {
                out.push(ch.to_ascii_uppercase());
            } else {
                out.push(ch.to_ascii_lowercase());
            }
            capitalize = false;
        } else {
            capitalize = true;
        }
    }

    if out.is_empty() {
        return "Model".to_string();
    }
    if out
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        out = format!("Model{out}");
    }
    out
}

fn build_whitelist() -> HashMap<String, HashSet<usize>> {
    HashMap::from([
        ("vocabulary.json".to_string(), HashSet::from([0usize])),
        ("properties.json".to_string(), HashSet::from([2usize])),
        ("default.json".to_string(), HashSet::from([0usize])),
    ])
}

fn is_whitelisted(map: &HashMap<String, HashSet<usize>>, file: &str, idx: usize) -> bool {
    map.get(file).map(|s| s.contains(&idx)).unwrap_or(false)
}

fn allows_non_object_inputs(schema: &Value) -> bool {
    match schema {
        Value::Object(map) => match map.get("type") {
            None => true,
            Some(Value::String(t)) => t != "object",
            Some(Value::Array(types)) => {
                let mut has_object = false;
                let mut has_other = false;
                for ty in types {
                    if ty.as_str() == Some("object") {
                        has_object = true;
                    } else {
                        has_other = true;
                    }
                }
                has_other || !has_object
            }
            _ => false,
        },
        _ => false,
    }
}

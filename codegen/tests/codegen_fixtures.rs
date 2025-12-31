use json_schema_ast::build_and_resolve_schema;
use json_schema_codegen::{pydantic, CodegenError, ModelRole, PydanticOptions};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn fixtures_pydantic_goldens() -> Result<(), Box<dyn std::error::Error>> {
    let regen = std::env::var_os("REGEN_CODEGEN_GOLDENS").is_some();
    let base_module = "json_schema_codegen_base";
    let base_code = pydantic::base_module();

    let fixtures_root = PathBuf::from("../tests/fixtures/fuzz");
    let golden_root = PathBuf::from("tests/golden/pydantic");
    let whitelist = load_whitelist(&golden_root)?;

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
                || name_str == ".venv"
                || name_str == ".uv_cache"
                || name_str == ".pytest_cache"
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

        for (schema_json, idx, tests) in schemas {
            let golden_rel_path = strip_json_extension(&rel_path);
            if is_whitelisted(&whitelist, &golden_rel_path, idx) {
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
            let options = PydanticOptions::default()
                .with_root_model_name(root_name.clone())
                .with_base_module(base_module)
                .with_header_comment(format_header_comment(&schema_json, &tests));
            let serializer =
                match pydantic::generate_model(&schema, ModelRole::Serializer, options.clone()) {
                    Ok(code) => code,
                    Err(CodegenError::RootNotObject { .. }) => continue,
                    Err(CodegenError::UnsupportedFeature { .. }) => continue,
                    Err(err) => return Err(format!("{rel_path}#{idx} serializer: {err}").into()),
                };
            let deserializer =
                match pydantic::generate_model(&schema, ModelRole::Deserializer, options) {
                    Ok(code) => code,
                    Err(CodegenError::RootNotObject { .. }) => continue,
                    Err(CodegenError::UnsupportedFeature { .. }) => continue,
                    Err(err) => return Err(format!("{rel_path}#{idx} deserializer: {err}").into()),
                };

            let base_dir = golden_root.join(&golden_rel_path);
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

fn collect_schemas(root: &Value) -> Vec<(Value, usize, Vec<Value>)> {
    match root {
        Value::Array(groups) => {
            let mut out = Vec::new();
            for (idx, item) in groups.iter().enumerate() {
                if let Some(schema) = item.get("schema") {
                    let tests = item
                        .get("tests")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default();
                    out.push((schema.clone(), idx, tests));
                }
            }
            if out.is_empty() {
                vec![(root.clone(), 0, Vec::new())]
            } else {
                out
            }
        }
        other => {
            let tests = other
                .get("tests")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            vec![(other.clone(), 0, tests)]
        }
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

fn is_whitelisted(map: &HashMap<String, HashSet<usize>>, file: &str, idx: usize) -> bool {
    map.get(file).map(|s| s.contains(&idx)).unwrap_or(false)
}

fn load_whitelist(
    golden_root: &Path,
) -> Result<HashMap<String, HashSet<usize>>, Box<dyn std::error::Error>> {
    let path = golden_root.join("tests").join("whitelist.json");
    let text = fs::read_to_string(&path)?;
    let raw: HashMap<String, HashMap<String, String>> = serde_json::from_str(&text)?;
    let mut out: HashMap<String, HashSet<usize>> = HashMap::new();

    for (file, entries) in raw {
        let file = strip_json_extension(&file);
        let mut set = HashSet::new();
        for idx in entries.keys() {
            if let Ok(val) = idx.parse::<usize>() {
                set.insert(val);
            }
        }
        if !set.is_empty() {
            out.insert(file, set);
        }
    }

    Ok(out)
}

fn strip_json_extension(rel_path: &str) -> String {
    let path = Path::new(rel_path);
    if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
        path.with_extension("")
            .to_string_lossy()
            .replace('\\', "/")
            .to_string()
    } else {
        rel_path.to_string()
    }
}

fn format_header_comment(schema: &Value, tests: &[Value]) -> String {
    let mut out = String::new();
    out.push_str("Schema:\n");
    match serde_json::to_string_pretty(schema) {
        Ok(s) => out.push_str(&s),
        Err(_) => out.push_str("<unserializable schema>"),
    }
    out.push_str("\n\nTests:\n");
    if tests.is_empty() {
        out.push_str("[]");
    } else {
        match serde_json::to_string_pretty(tests) {
            Ok(s) => out.push_str(&s),
            Err(_) => out.push_str("<unserializable tests>"),
        }
    }
    out.push('\n');
    out
}

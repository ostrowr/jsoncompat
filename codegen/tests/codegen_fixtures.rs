use json_schema_ast::build_and_resolve_schema;
use json_schema_codegen::{pydantic, CodegenError, ModelRole, PydanticOptions};
use serde_json::Map as JsonMap;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, Once, OnceLock};

const FIXTURES_ROOT: &str = "../tests/fixtures/fuzz";
const GOLDEN_ROOT: &str = "tests/golden/pydantic";
const BASE_MODULE: &str = "json_schema_codegen_base";
static WHITELIST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

datatest_stable::harness!(fixture, FIXTURES_ROOT, ".*\\.json$");

fn fixture(file: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let regen = std::env::var_os("REGEN_CODEGEN_GOLDENS").is_some();
    let base_code = pydantic::base_module();
    let fixtures_root = PathBuf::from(FIXTURES_ROOT);
    let golden_root = PathBuf::from(GOLDEN_ROOT);
    ensure_initialized(&golden_root, base_code, regen);

    let whitelist = load_whitelist(&golden_root)?;
    let rel_path = file.strip_prefix(&fixtures_root).unwrap_or(file);
    let rel_str = rel_path.to_string_lossy().replace('\\', "/");

    let content = fs::read(file)?;
    let root: Value = serde_json::from_slice(&content)?;
    let schemas = collect_schemas(&root);

    for (schema_json, idx, tests) in schemas {
        let golden_rel_path = strip_json_extension(&rel_str);
        if !regen && is_whitelisted(&whitelist, &golden_rel_path, idx) {
            continue;
        }
        let schema =
            build_and_resolve_schema(&schema_json).map_err(|e| format!("{rel_str}#{idx}: {e}"))?;
        let root_name = format!(
            "{}{}",
            sanitize_type_name(
                file.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .as_ref()
            ),
            idx
        );
        let options = PydanticOptions::default()
            .with_root_model_name(root_name.clone())
            .with_base_module(BASE_MODULE)
            .with_header_comment(format_header_comment(&schema_json, &tests));
        let serializer =
            match pydantic::generate_model(&schema, ModelRole::Serializer, options.clone()) {
                Ok(code) => code,
                Err(err) => {
                    if regen {
                        update_whitelist(&golden_root, &golden_rel_path, idx, &err.to_string())?;
                    }
                    stub_model(ModelRole::Serializer, &options, &root_name, &err)
                }
            };
        let deserializer =
            match pydantic::generate_model(&schema, ModelRole::Deserializer, options.clone()) {
                Ok(code) => code,
                Err(err) => {
                    if regen {
                        update_whitelist(&golden_root, &golden_rel_path, idx, &err.to_string())?;
                    }
                    stub_model(ModelRole::Deserializer, &options, &root_name, &err)
                }
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
            "Serializer golden mismatch for {rel_str}#{idx}; set REGEN_CODEGEN_GOLDENS=1 to refresh"
        );
        assert_eq!(
            deserializer, expected_deserializer,
            "Deserializer golden mismatch for {rel_str}#{idx}; set REGEN_CODEGEN_GOLDENS=1 to refresh"
        );
    }

    Ok(())
}

fn ensure_initialized(golden_root: &Path, base_code: String, regen: bool) {
    static INIT: Once = Once::new();

    let golden_root = golden_root.to_path_buf();
    INIT.call_once(|| {
        if let Err(err) = initialize_goldens(&golden_root, BASE_MODULE, &base_code, regen) {
            panic!("Failed to initialize codegen goldens: {err}");
        }
    });
}

fn stub_model(
    role: ModelRole,
    options: &PydanticOptions,
    root_name: &str,
    err: &CodegenError,
) -> String {
    let class_name = match role {
        ModelRole::Serializer => format!("{root_name}{}", options.serializer_suffix),
        ModelRole::Deserializer => format!("{root_name}{}", options.deserializer_suffix),
    };

    let mut out = String::new();
    if let Some(comment) = &options.header_comment {
        out.push_str("\"\"\"\n");
        out.push_str(comment);
        if !comment.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("\"\"\"\n\n");
    }

    out.push_str("from pydantic import BaseModel, ConfigDict\n\n");
    out.push_str(&format!("class {class_name}(BaseModel):\n"));
    out.push_str("    model_config = ConfigDict(extra=\"forbid\")\n\n");
    out.push_str("    @classmethod\n");
    out.push_str("    def __get_pydantic_core_schema__(cls, source, handler):\n");
    out.push_str(&format!(
        "        raise NotImplementedError({:?})\n",
        err.to_string()
    ));
    out
}

fn initialize_goldens(
    golden_root: &Path,
    base_module: &str,
    base_code: &str,
    regen: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if regen && golden_root.exists() {
        for entry in fs::read_dir(golden_root)? {
            let entry = entry?;
            let path = entry.path();
            let name_str = entry.file_name().to_string_lossy().to_string();
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
        fs::write(&base_path, base_code)?;
    } else {
        let expected_base = fs::read_to_string(&base_path).map_err(|_| {
            format!(
                "Missing base module golden {}; set REGEN_CODEGEN_GOLDENS=1 to refresh",
                base_path.display()
            )
        })?;
        if base_code != expected_base {
            return Err(
                "Base module golden mismatch; set REGEN_CODEGEN_GOLDENS=1 to refresh".into(),
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
    let guard = WHITELIST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap();
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

    drop(guard);
    Ok(out)
}

fn update_whitelist(
    golden_root: &Path,
    golden_rel_path: &str,
    idx: usize,
    reason: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let guard = WHITELIST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap();

    let path = golden_root.join("tests").join("whitelist.json");
    let mut root: JsonMap<String, Value> = if path.exists() {
        serde_json::from_str(&fs::read_to_string(&path)?)?
    } else {
        JsonMap::new()
    };

    let mut file_entry = root
        .remove(golden_rel_path)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    file_entry.insert(idx.to_string(), Value::String(reason.to_string()));
    root.insert(golden_rel_path.to_string(), Value::Object(file_entry));

    let sorted = sort_object_keys(root);
    let serialized = serde_json::to_string_pretty(&Value::Object(sorted))?;
    fs::write(path, format!("{serialized}\n"))?;
    drop(guard);
    Ok(())
}

fn sort_object_keys(obj: JsonMap<String, Value>) -> JsonMap<String, Value> {
    let mut entries: Vec<_> = obj.into_iter().collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut out = JsonMap::new();
    for (key, value) in entries {
        let value = match value {
            Value::Object(inner) => Value::Object(sort_object_keys(inner)),
            Value::Array(items) => Value::Array(
                items
                    .into_iter()
                    .map(|item| match item {
                        Value::Object(inner) => Value::Object(sort_object_keys(inner)),
                        other => other,
                    })
                    .collect(),
            ),
            other => other,
        };
        out.insert(key, value);
    }

    out
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

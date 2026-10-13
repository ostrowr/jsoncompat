//! Differential coverage for the native HTTP body path and its value fallback.

use serde_json::{Value, json};
use std::{fs, process::Command};
#[path = "support/python_env.rs"]
mod python_env;

#[test]
fn native_http_bodies_preserve_value_conversion_semantics() {
    let directory = std::env::temp_dir().join(format!(
        "jsoncompat-http-native-{}-{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    fs::create_dir(&directory).unwrap();
    let schemas = json!({
        "Integer": {"type": "integer", "minimum": 0},
        "Number": {"type": "number", "minimum": 0.1, "maximum": 0.3},
        "Bag": {
            "type": "object", "properties": {"fixed": {"type": "integer"}},
            "additionalProperties": {"type": "number"}
        },
        "Union": {"anyOf": [
            {"type": "string", "minLength": 1},
            {"type": "object", "properties": {
                "kind": {"const": "list"},
                "items": {"type": "array", "items": {"type": "integer", "minimum": 0}}
            }, "required": ["kind", "items"], "additionalProperties": false}
        ]},
        "Array": {"type": "array", "items": {"type": ["number", "null"]}},
        "Anything": {}
    });
    let mut paths = serde_json::Map::new();
    for name in schemas.as_object().unwrap().keys() {
        let schema = json!({"$ref": format!("#/components/schemas/{name}")});
        paths.insert(format!("/{}", name.to_lowercase()), json!({
            "post": {
                "operationId": name,
                "parameters": [{"name": "filter", "in": "query", "style": "deepObject", "explode": true,
                    "schema": {"type": "object", "additionalProperties": {"type": "integer"}}}],
                "requestBody": {"required": true, "content": {"application/json": {"schema": schema}}},
                "responses": {"200": {
                    "description": "Native body", "headers": {
                        "X-Count": {"schema": {"type": "integer"}},
                        "X-Map": {"content": {"application/json": {"schema": {"type": "object", "additionalProperties": {"type": "integer"}}}}}
                    }, "content": {"application/json": {"schema": schema}}
                }}
            }
        }));
    }
    paths.insert("/variants".into(), json!({"post": {
        "operationId": "Variants",
        "requestBody": {"required": true, "content": {
            "application/json": {"schema": {"$ref": "#/components/schemas/Bag"}},
            "text/plain": {"schema": {"type": "string"}}
        }},
        "responses": {"200": {"description": "Done", "content": {"application/json": {"schema": {}}}}}
    }}));
    let specification = json!({
        "openapi": "3.1.0", "info": {"title": "Native HTTP", "version": "1"},
        "paths": Value::Object(paths), "components": {"schemas": schemas}
    });
    fs::write(directory.join("openapi.json"), specification.to_string()).unwrap();
    let generated = Command::new(env!("CARGO_BIN_EXE_jsoncompat"))
        .args([
            "codegen",
            "--openapi",
            "openapi.json",
            "--api",
            "Native",
            "--output",
            "http_native",
        ])
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let tested = python_env::python_command()
        .arg(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/python/openapi_http_native.py"),
        )
        .arg(&directory)
        .current_dir(&directory)
        .output()
        .unwrap();
    fs::remove_dir_all(&directory).unwrap();
    assert!(
        tested.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&tested.stdout),
        String::from_utf8_lossy(&tested.stderr)
    );
}

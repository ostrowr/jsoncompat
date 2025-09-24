use wasm_bindgen::prelude::*;

use json_schema_fuzz::{generate_value, GenerateError};
use jsoncompat::{build_and_resolve_schema, check_compat, Role};

use rand::thread_rng;
use serde_json::Value as JsonValue;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn parse_json(s: &str) -> Result<JsonValue, JsValue> {
    serde_json::from_str(s).map_err(|e| JsValue::from_str(&format!("invalid JSON: {e}")))
}

fn parse_role(role: &str) -> Result<Role, JsValue> {
    match role.to_ascii_lowercase().as_str() {
        "serializer" => Ok(Role::Serializer),
        "deserializer" => Ok(Role::Deserializer),
        "both" => Ok(Role::Both),
        _ => Err(JsValue::from_str(
            "role must be 'serializer', 'deserializer' or 'both'",
        )),
    }
}

/// Check compatibility between two schemas.
///
/// * `old_schema_json` – original schema as JSON string
/// * `new_schema_json` – updated schema as JSON string
/// * `role` – "serializer", "deserializer" or "both" (default)
/// Exported to JavaScript as `check_compat`.
#[wasm_bindgen(js_name = check_compat)]
pub fn check_compat_js(
    old_schema_json: &str,
    new_schema_json: &str,
    role: &str,
) -> Result<bool, JsValue> {
    let role_e = parse_role(role)?;
    let old_raw = parse_json(old_schema_json)?;
    let new_raw = parse_json(new_schema_json)?;

    let old_ast = build_and_resolve_schema(&old_raw)
        .map_err(|e| JsValue::from_str(&format!("invalid old schema: {e}")))?;
    let new_ast = build_and_resolve_schema(&new_raw)
        .map_err(|e| JsValue::from_str(&format!("invalid new schema: {e}")))?;

    Ok(check_compat(&old_ast, &new_ast, role_e))
}

/// Generate a JSON value (string) that should satisfy the given schema.
///
/// * `schema_json` – schema as JSON string
/// * `depth` – recursion depth (default 5)
/// Exported to JavaScript as `generate_value`.
#[wasm_bindgen(js_name = generate_value)]
pub fn generate_value_js(schema_json: &str, depth: u8) -> Result<String, JsValue> {
    let raw = parse_json(schema_json)?;
    let schema_ast = build_and_resolve_schema(&raw)
        .map_err(|e| JsValue::from_str(&format!("invalid schema: {e}")))?;

    let mut rng = thread_rng();
    let v = match generate_value(&schema_ast, &mut rng, depth) {
        Ok(v) => v,
        Err(GenerateError::Unsatisfiable) => {
            return Err(JsValue::from_str("schema has no valid instances"));
        }
        Err(GenerateError::Exhausted) => {
            return Err(JsValue::from_str(
                "failed to generate a value that satisfies the schema",
            ));
        }
    };
    serde_json::to_string(&v).map_err(|e| JsValue::from_str(&format!("serialization failure: {e}")))
}

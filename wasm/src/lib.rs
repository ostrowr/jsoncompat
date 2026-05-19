//! WebAssembly bindings for the `jsoncompat` compatibility checker and value generator.
//!
//! JavaScript callers get two exported functions: `check_compat` and
//! `generate_value`. Both accept schemas as JSON strings and return JavaScript
//! values or string errors through `wasm-bindgen`.

use wasm_bindgen::prelude::*;

use json_schema_fuzz::{GenerateError, GenerationConfig, ValueGenerator};
use jsoncompat::{Role, SchemaDocument, check_compat, validate_compatibility_input};

use serde_json::Value as JsonValue;

fn validated_schema(raw: &JsonValue) -> Result<SchemaDocument, String> {
    let schema = SchemaDocument::from_json(raw).map_err(|error| error.to_string())?;
    schema.root().map_err(|error| error.to_string())?;
    schema
        .validate_source_schema()
        .map_err(|error| error.to_string())?;
    Ok(schema)
}

fn compatibility_schema(raw: &JsonValue) -> Result<SchemaDocument, String> {
    let schema = SchemaDocument::from_json(raw).map_err(|error| error.to_string())?;
    validate_compatibility_input(&schema).map_err(|error| error.to_string())?;
    Ok(schema)
}

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
/// * `role` – "serializer", "deserializer" or "both"
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

    let old_schema = compatibility_schema(&old_raw)
        .map_err(|e| JsValue::from_str(&format!("invalid old schema: {e}")))?;
    let new_schema = compatibility_schema(&new_raw)
        .map_err(|e| JsValue::from_str(&format!("invalid new schema: {e}")))?;

    check_compat(&old_schema, &new_schema, role_e)
        .map_err(|e| JsValue::from_str(&format!("compatibility check failed: {e}")))
}

/// Generate a JSON value (string) that should satisfy the given schema.
///
/// * `schema_json` – schema as JSON string
/// * `depth` – recursion depth limit
/// Exported to JavaScript as `generate_value`.
#[wasm_bindgen(js_name = generate_value)]
pub fn generate_value_js(schema_json: &str, depth: u8) -> Result<String, JsValue> {
    let raw = parse_json(schema_json)?;
    let schema =
        validated_schema(&raw).map_err(|e| JsValue::from_str(&format!("invalid schema: {e}")))?;

    let mut rng = rand::rng();
    let v = ValueGenerator::generate(&schema, GenerationConfig::new(depth), &mut rng).map_err(
        |error| match error {
            GenerateError::Schema(error) => JsValue::from_str(&format!("invalid schema: {error}")),
            GenerateError::Unsatisfiable => JsValue::from_str(&error.to_string()),
            GenerateError::ExhaustedAttempts { .. } => JsValue::from_str(&error.to_string()),
            _ => JsValue::from_str(&error.to_string()),
        },
    )?;
    serde_json::to_string(&v).map_err(|e| JsValue::from_str(&format!("serialization failure: {e}")))
}

#[cfg(test)]
mod tests {
    use super::{compatibility_schema, validated_schema};
    use serde_json::json;

    #[test]
    fn compatibility_schema_validation_accepts_unmodeled_keywords_for_modeled_comparison() {
        compatibility_schema(&json!({
            "type": "object",
            "dependentSchemas": {
                "kind": { "required": ["detail"] }
            }
        }))
        .expect("compatibility bindings should accept warning-only schema keywords");
    }

    #[test]
    fn generation_schema_validation_rejects_backend_invalid_ref_bearing_schemas_up_front() {
        let error = validated_schema(&json!({
            "$defs": {
                "Value": { "type": "string" }
            },
            "$ref": "#/$defs/Value",
            "deprecated": "eventually"
        }))
        .expect_err("generation bindings must validate raw ref-bearing schemas before work");

        assert!(
            error.contains("schema failed Draft 2020-12 validator compilation"),
            "unexpected error: {error}"
        );
    }
}

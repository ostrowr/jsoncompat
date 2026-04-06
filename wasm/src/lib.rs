//! WebAssembly bindings for the `jsoncompat` compatibility checker and value generator.
//!
//! JavaScript callers get exported compatibility helpers plus reusable
//! validators and generators. Public functions accept schemas as JSON strings
//! and return JavaScript values or string errors through `wasm-bindgen`.

use wasm_bindgen::prelude::*;

use json_schema_fuzz::{GenerateError, GenerationConfig, ValueGenerator};
use jsoncompat::{Role, SchemaDocument, check_compat};

use serde_json::Value as JsonValue;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn parse_json(s: &str) -> Result<JsonValue, JsValue> {
    serde_json::from_str(s).map_err(|e| JsValue::from_str(&format!("invalid JSON: {e}")))
}

fn parse_schema(schema_json: &str) -> Result<SchemaDocument, JsValue> {
    let raw = parse_json(schema_json)?;
    SchemaDocument::from_json(&raw).map_err(|e| JsValue::from_str(&format!("invalid schema: {e}")))
}

fn generate_value_for_schema(schema: &SchemaDocument, depth: u8) -> Result<String, JsValue> {
    let mut rng = rand::rng();
    let value = ValueGenerator::generate(schema, GenerationConfig::new(depth), &mut rng).map_err(
        |error| match error {
            GenerateError::Schema(error) => JsValue::from_str(&format!("invalid schema: {error}")),
            GenerateError::Unsatisfiable => JsValue::from_str(&error.to_string()),
            GenerateError::ExhaustedAttempts { .. } => JsValue::from_str(&error.to_string()),
            _ => JsValue::from_str(&error.to_string()),
        },
    )?;
    serde_json::to_string(&value)
        .map_err(|e| JsValue::from_str(&format!("serialization failure: {e}")))
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

/// Reusable validator for one JSON Schema document.
#[wasm_bindgen]
pub struct Validator {
    schema: SchemaDocument,
}

#[wasm_bindgen]
impl Validator {
    /// Check whether a JSON value satisfies this validator's schema.
    #[wasm_bindgen(js_name = is_valid)]
    pub fn is_valid_js(&self, instance_json: &str) -> Result<bool, JsValue> {
        let instance = parse_json(instance_json)?;
        self.schema
            .is_valid(&instance)
            .map_err(|e| JsValue::from_str(&format!("validation failed: {e}")))
    }
}

/// Reusable generator for one JSON Schema document.
#[wasm_bindgen]
pub struct Generator {
    schema: SchemaDocument,
}

#[wasm_bindgen]
impl Generator {
    /// Generate a JSON value that should satisfy this generator's schema.
    #[wasm_bindgen(js_name = generate_value)]
    pub fn generate_value_js(&self, depth: u8) -> Result<String, JsValue> {
        generate_value_for_schema(&self.schema, depth)
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

    let old_schema = SchemaDocument::from_json(&old_raw)
        .map_err(|e| JsValue::from_str(&format!("invalid old schema: {e}")))?;
    let new_schema = SchemaDocument::from_json(&new_raw)
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
    generate_value_for_schema(&parse_schema(schema_json)?, depth)
}

/// Build a reusable generator for a JSON Schema.
///
/// * `schema_json` – schema as JSON string
/// Exported to JavaScript as `generator_for`.
#[wasm_bindgen(js_name = generator_for)]
pub fn generator_for_js(schema_json: &str) -> Result<Generator, JsValue> {
    Ok(Generator {
        schema: parse_schema(schema_json)?,
    })
}

/// Build a reusable validator for a JSON Schema.
///
/// * `schema_json` – schema as JSON string
/// Exported to JavaScript as `validator_for`.
#[wasm_bindgen(js_name = validator_for)]
pub fn validator_for_js(schema_json: &str) -> Result<Validator, JsValue> {
    Ok(Validator {
        schema: parse_schema(schema_json)?,
    })
}

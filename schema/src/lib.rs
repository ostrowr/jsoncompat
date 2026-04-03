//! A thin convenience wrapper that exposes strict Draft 2020‑12 validation
//! and a Schema AST builder.

pub mod ast;

pub use ast::{
    SchemaNode, SchemaNodeKind, build_and_resolve_schema, build_schema_ast, resolve_refs,
};

use anyhow::{Context, Result};
use jsonschema::Draft;
pub use jsonschema::JSONSchema;
use serde_json::{Map, Value};

/// Compile the provided raw JSON Schema into the proven validator, enforcing
/// Draft 2020‑12 semantics.  Higher‑level crates use this to avoid relying on
/// the partial validator that was in place during prototyping.
pub fn compile(schema: &Value) -> Result<JSONSchema> {
    // The `jsonschema` crate keeps references to the original schema tree
    // inside the compiled validator, therefore the value passed in must live
    // for `'static`.  We perform a light‑weight clone and leak it – acceptable
    // for short‑running test/fuzz sessions.
    let owned: Value = schema.clone();
    let static_ref: &'static Value = Box::leak(Box::new(owned));
    JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(static_ref)
        .context("Failed to compile JSON Schema")
}

/// Return a recursively canonicalized JSON value with object keys sorted.
pub fn canonicalize_json(value: &Value) -> Value {
    match value {
        Value::Object(obj) => {
            let mut canonical = Map::new();
            let mut keys = obj.keys().collect::<Vec<_>>();
            keys.sort();
            for key in keys {
                let child = obj.get(key).expect("key from map");
                canonical.insert(key.clone(), canonicalize_json(child));
            }
            Value::Object(canonical)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_json).collect()),
        _ => value.clone(),
    }
}

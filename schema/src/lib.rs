//! A thin convenience wrapper that exposes strict Draft 2020‑12 validation
//! and a Schema AST builder.

pub mod ast;

pub use ast::{build_and_resolve_schema, build_schema_ast, resolve_refs, SchemaNode};

use anyhow::{Context, Result};
use jsonschema::Draft;
pub use jsonschema::JSONSchema;
use serde_json::Value;

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

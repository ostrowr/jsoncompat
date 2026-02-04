//! A thin convenience wrapper that exposes strict Draft 2020‑12 validation
//! and a Schema AST builder.

pub mod ast;

pub use ast::{
    SchemaNode, SchemaNodeKind, build_and_resolve_schema, build_schema_ast, resolve_refs,
};

use anyhow::{Context, Result};
use jsonschema::Draft;
pub use jsonschema::Validator as JSONSchema;
use serde_json::Value;

/// Compile the provided raw JSON Schema into the proven validator, enforcing
/// Draft 2020‑12 semantics.  Higher‑level crates use this to avoid relying on
/// the partial validator that was in place during prototyping.
pub fn compile(schema: &Value) -> Result<JSONSchema> {
    JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .build(schema)
        .context("Failed to compile JSON Schema")
}

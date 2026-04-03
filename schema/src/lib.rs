//! A thin convenience wrapper that exposes strict Draft 2020‑12 validation
//! and a Schema AST builder.

mod ast;
mod canonicalize;

mod schema_metadata;

pub use ast::{AstError, SchemaNode, SchemaNodeKind, build_and_resolve_schema};
pub use canonicalize::CanonicalizeError as SchemaError;

use canonicalize::{CanonicalSchema, canonicalize_schema};
use jsonschema::Draft;
pub use jsonschema::JSONSchema;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CompileError {
    #[error(transparent)]
    Schema(#[from] SchemaError),
    #[error("canonical schema failed Draft 2020-12 validator compilation: {source}")]
    ValidatorRejectedSchema {
        #[source]
        source: Box<jsonschema::ValidationError<'static>>,
    },
}
/// Compile a JSON Schema document after normalizing it into the internal
/// representation used by the AST builder and subset checker.
pub fn compile(schema: &Value) -> Result<JSONSchema, CompileError> {
    let canonical = canonicalize_schema(schema)?;
    compile_canonical(&canonical)
}

fn compile_canonical(schema: &CanonicalSchema) -> Result<JSONSchema, CompileError> {
    // The `jsonschema` crate keeps references to the original schema tree
    // inside the compiled validator, therefore the value passed in must live
    // for `'static`.  We perform a light‑weight clone and leak it – acceptable
    // for short‑running test/fuzz sessions.
    let owned = schema.as_value().clone();
    let static_ref: &'static serde_json::Value = Box::leak(Box::new(owned));
    JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(static_ref)
        .map_err(|source| CompileError::ValidatorRejectedSchema {
            source: Box::new(source),
        })
}

#[cfg(test)]
mod roundtrip_tests;

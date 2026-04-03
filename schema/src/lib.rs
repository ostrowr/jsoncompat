//! A thin convenience wrapper that exposes strict Draft 2020‑12 validation
//! and a Schema AST builder.

pub mod ast;
pub mod canonicalize;

mod schema_metadata;

pub use ast::{
    AstError, SchemaNode, SchemaNodeKind, build_and_resolve_canonical_schema,
    build_canonical_schema_ast, resolve_refs,
};
pub use canonicalize::{
    CanonicalSchema, CanonicalizeError, canonicalize_json, canonicalize_schema,
};

use jsonschema::Draft;
pub use jsonschema::JSONSchema;

#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("canonical schema failed Draft 2020-12 validator compilation: {source}")]
    ValidatorRejectedSchema {
        #[source]
        source: Box<jsonschema::ValidationError<'static>>,
    },
}
/// Compile an already canonicalized JSON Schema document.
pub fn compile_canonical(schema: &CanonicalSchema) -> Result<JSONSchema, CompileError> {
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

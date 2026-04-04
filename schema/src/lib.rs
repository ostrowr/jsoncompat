//! A thin convenience wrapper that exposes strict Draft 2020‑12 validation
//! and a Schema AST builder.

mod ast;
mod canonicalize;

mod schema_metadata;

pub use ast::{
    ArrayContains, AstError, ResolvedNode, ResolvedNodeId, ResolvedNodeKind, ResolvedSchema,
    SchemaBuildError, SchemaNode, SchemaNodeId, build_and_resolve_schema,
};
pub use canonicalize::CanonicalizeError as SchemaError;

#[cfg(test)]
use canonicalize::CanonicalSchema;
use canonicalize::validate_schema_dialects;
use jsonschema::Draft;
pub use jsonschema::JSONSchema;
use serde_json::Value;
use std::borrow::Cow;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CompileError {
    #[error(transparent)]
    Schema(#[from] SchemaError),
    #[error("schema failed Draft 2020-12 validator compilation: {source}")]
    ValidatorRejectedSchema {
        #[source]
        source: Box<jsonschema::ValidationError<'static>>,
    },
}
/// Compile a JSON Schema document directly with the validator backend.
pub fn compile(schema: &Value) -> Result<JSONSchema, CompileError> {
    validate_schema_dialects(schema)?;
    compile_schema_value(schema)
}

#[cfg(test)]
pub(crate) fn compile_canonical(schema: &CanonicalSchema) -> Result<JSONSchema, CompileError> {
    compile_schema_value(schema.as_value())
}

fn compile_schema_value(schema: &Value) -> Result<JSONSchema, CompileError> {
    // `jsonschema::JSONSchema` owns the compiled validation tree, but schema
    // compilation errors borrow the rejected schema fragment. Convert those
    // failures into owned errors before returning so callers do not need to
    // keep the original `Value` alive.
    JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(schema)
        .map_err(|source| CompileError::ValidatorRejectedSchema {
            source: Box::new(owned_validation_error(source)),
        })
}

fn owned_validation_error(
    source: jsonschema::ValidationError<'_>,
) -> jsonschema::ValidationError<'static> {
    jsonschema::ValidationError {
        instance: Cow::Owned(source.instance.into_owned()),
        kind: source.kind,
        instance_path: source.instance_path,
        schema_path: source.schema_path,
    }
}

#[cfg(test)]
mod roundtrip_tests;

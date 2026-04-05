//! Strict Draft 2020-12 schema documents, validation, and resolved schema IR.
//!
//! The main entry point is [`SchemaDocument`]. Build one from raw JSON with
//! [`SchemaDocument::from_json`], validate instances with [`SchemaDocument::is_valid`],
//! and use [`compile`] only when you need direct access to the underlying
//! validator backend. Crates that implement analysis or generation can inspect
//! the lazily resolved canonical IR with [`SchemaDocument::root`].

mod ast;
mod canonicalize;
mod constraints;
mod json_semantics;

mod schema_metadata;

#[cfg(test)]
pub(crate) use ast::build_and_resolve_schema;
pub use ast::{
    AstError, IntegerMultipleOf, NodeId, NumberMultipleOf, SchemaBuildError, SchemaDocument,
    SchemaNode, SchemaNodeKind,
};
pub use canonicalize::CanonicalizeError as SchemaError;
pub use constraints::{
    ContainsConstraint, CountRange, IntegerBounds, NumberBound, NumberBounds, PatternConstraint,
    PatternProperty, PatternSupport,
};
pub use json_semantics::json_values_equal;

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
    /// The raw schema failed this crate's dialect or keyword-shape checks.
    #[error(transparent)]
    Schema(#[from] SchemaError),
    /// The `jsonschema` backend rejected the schema after local checks passed.
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

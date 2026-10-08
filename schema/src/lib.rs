//! Strict Draft 2020-12 JSON Schema and OpenAPI 3.1 Schema Object documents,
//! validation, and resolved schema IR.
//!
//! The main entry point is [`SchemaDocument`]. Build one from raw JSON with
//! [`SchemaDocument::from_json`], validate instances with [`SchemaDocument::is_valid`],
//! and use [`compile`] only when you need direct access to the underlying
//! validator backend. Crates that implement analysis or generation can inspect
//! the lazily resolved canonical IR with [`SchemaDocument::root`].

mod ast;
mod canonicalize;
mod constraints;
mod exact;
mod options;
pub use options::SchemaOptions;
mod references;
mod unevaluated;
mod validation;
pub use exact::ExactNumber;
pub use validation::ValidationConstraint;
mod json_semantics;
mod schema_children;

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
pub use schema_children::{
    SCHEMA_ARRAY_CHILD_KEYWORDS, SCHEMA_MAP_CHILD_KEYWORDS, SINGLE_SCHEMA_CHILD_KEYWORDS,
    is_schema_array_child_keyword, is_schema_map_child_keyword, is_single_schema_child_keyword,
};

#[cfg(test)]
use canonicalize::CanonicalSchema;
use canonicalize::validate_schema_dialects;
pub use jsonschema::Validator as JSONSchema;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CompileError {
    #[error("schema reference resolution failed: {source}")]
    Reference {
        #[source]
        source: Box<AstError>,
    },
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
    compile_schema_value(prepare_for_validation(schema)?.as_ref())
}

/// Prepare an equivalent backend input without traversing annotation-only
/// content references. Referenced resources inside contentSchema remain usable.
pub fn prepare_for_validation(schema: &Value) -> Result<std::borrow::Cow<'_, Value>, CompileError> {
    fn has_content(schema: &Value) -> bool {
        schema.get("contentSchema").is_some()
            || references::children(schema)
                .iter()
                .any(|(_, child)| has_content(child))
    }
    if !has_content(schema) {
        return Ok(std::borrow::Cow::Borrowed(schema));
    }
    jsonschema::draft202012::meta::validate(schema).map_err(|source| {
        CompileError::ValidatorRejectedSchema {
            source: Box::new(source.to_owned()),
        }
    })?;
    let mut linked = references::link(schema).map_err(|source| CompileError::Reference {
        source: Box::new(source),
    })?;
    fn strip(schema: &mut Value) {
        if let Some(object) = schema.as_object_mut() {
            object.remove("contentSchema");
        }
        let paths: Vec<_> = references::children(schema)
            .into_iter()
            .map(|(path, _)| path)
            .collect();
        for path in paths {
            if let Some(child) = schema.pointer_mut(&format!("/{path}")) {
                strip(child);
            }
        }
    }
    strip(&mut linked);
    Ok(std::borrow::Cow::Owned(linked))
}

#[cfg(test)]
pub(crate) fn compile_canonical(schema: &CanonicalSchema) -> Result<JSONSchema, CompileError> {
    compile_schema_value(prepare_for_validation(schema.as_value())?.as_ref())
}

fn compile_schema_value(schema: &Value) -> Result<JSONSchema, CompileError> {
    // `jsonschema::JSONSchema` owns the compiled validation tree, but schema
    // compilation errors borrow the rejected schema fragment. Convert those
    // failures into owned errors before returning so callers do not need to
    // keep the original `Value` alive.
    options::configure(exact::configure(jsonschema::draft202012::options()))
        .build(schema)
        .map_err(|source| CompileError::ValidatorRejectedSchema {
            source: Box::new(owned_validation_error(source)),
        })
}

fn owned_validation_error(
    source: jsonschema::ValidationError<'_>,
) -> jsonschema::ValidationError<'static> {
    source.to_owned()
}

#[cfg(test)]
mod roundtrip_tests;

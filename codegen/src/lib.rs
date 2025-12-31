//! Code generation utilities for JSON Schema.

pub mod pydantic;

mod error;
mod model;
mod parser;
mod strings;

use json_schema_ast::SchemaNode;
use serde_json::Value;

pub use error::{CodegenError, SchemaPath};
pub use model::{
    AdditionalProperties, FieldSpec, LiteralValue, ModelGraph, ModelRole, ModelSpec, SchemaType,
    StringFormat,
};
pub use pydantic::{
    base_module as pydantic_base_module, generate_model as generate_pydantic_model,
    generate_model_from_value as generate_pydantic_model_from_value, PydanticGenerator,
    PydanticOptions,
};

/// High-level contract for language-specific code generators.
pub trait ModelGenerator {
    fn generate_model(&self, schema: &SchemaNode, role: ModelRole) -> Result<String, CodegenError>;
}

/// Build the model graph from a resolved schema node.
pub fn build_model_graph(schema: &SchemaNode, root_name: &str) -> Result<ModelGraph, CodegenError> {
    let value = schema.to_json();
    build_model_graph_from_value(&value, root_name)
}

pub(crate) fn build_model_graph_from_value(
    schema: &Value,
    root_name: &str,
) -> Result<ModelGraph, CodegenError> {
    parser::ModelGraphBuilder::new(schema).build(root_name)
}

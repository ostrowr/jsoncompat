//! Code generation utilities for JSON Schema.

mod error;
mod model;
mod parser;
mod pydantic;
mod strings;

pub use error::{CodegenError, SchemaPath};
pub use model::{
    AdditionalProperties, FieldSpec, LiteralValue, ModelGraph, ModelRole, ModelSpec, SchemaType,
    StringFormat,
};
pub use pydantic::{PydanticGenerator, PydanticOptions};
use serde_json::Value;

/// Generator interface for producing a source file from a model graph.
pub trait CodeGenerator {
    fn generate(&self, graph: &ModelGraph) -> Result<String, CodegenError>;
}

/// Build the model graph from a JSON Schema document.
pub fn build_model_graph(schema: &Value, root_name: &str) -> Result<ModelGraph, CodegenError> {
    parser::ModelGraphBuilder::new(schema).build(root_name)
}

/// Generate code for a single model using the provided generator.
pub fn generate_model<G: CodeGenerator>(
    schema: &Value,
    root_name: &str,
    generator: &G,
) -> Result<String, CodegenError> {
    let graph = build_model_graph(schema, root_name)?;
    generator.generate(&graph)
}

/// Convenience helper for generating Pydantic v2 models.
pub fn generate_pydantic(
    schema: &Value,
    root_name: &str,
    options: PydanticOptions,
) -> Result<String, CodegenError> {
    let generator = PydanticGenerator::new(options);
    generate_model(schema, root_name, &generator)
}

 # json_schema_ast

 JSON Schema AST and reference resolver.

 [![crates.io](https://img.shields.io/crates/v/json_schema_ast)](https://crates.io/crates/json_schema_ast) [![docs.rs](https://docs.rs/json_schema_ast/badge.svg)](https://docs.rs/json_schema_ast) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)

 ## Installation

 Add to your `Cargo.toml`:

 ```toml
 [dependencies]
 json_schema_ast = "0.1.5"
 ```

 ## Usage

 ```rust
 use json_schema_ast::{build_and_resolve_schema, compile, SchemaNode, JSONSchema};
 use serde_json::json;

 let raw = json!({
     "type": "object",
     "properties": {
         "id": { "type": "integer" },
         "name": { "type": "string" }
     },
     "required": ["id"]
 });

 // Build AST
 let schema_node: SchemaNode = build_and_resolve_schema(&raw).unwrap();

 // Compile a fast validator
 let validator: JSONSchema = compile(&raw).unwrap();

 // Validate instances
 assert!(validator.is_valid(&json!({ "id": 42 })));
 ```

 ## License

 Licensed under MIT. See [LICENSE](../LICENSE).
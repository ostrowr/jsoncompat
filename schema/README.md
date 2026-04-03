# json_schema_ast

JSON Schema AST and reference resolver.

[![crates.io](https://img.shields.io/crates/v/json_schema_ast)](https://crates.io/crates/json_schema_ast) [![docs.rs](https://docs.rs/json_schema_ast/badge.svg)](https://docs.rs/json_schema_ast) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
json_schema_ast = "0.2.1"
```

## Usage

```rust
use json_schema_ast::{
    SchemaNode, JSONSchema, build_and_resolve_canonical_schema, canonicalize_schema,
    compile_canonical,
};
use serde_json::json;

let raw = json!({
    "type": "object",
    "properties": {
        "id": { "type": "integer" },
        "name": { "type": "string" }
    },
    "required": ["id"]
});

// Parse and canonicalize once at the boundary
let schema = canonicalize_schema(&raw).unwrap();

// Build AST
let schema_node: SchemaNode = build_and_resolve_canonical_schema(&schema).unwrap();

// Compile a fast validator
let validator: JSONSchema = compile_canonical(&schema).unwrap();

// Validate instances
assert!(validator.is_valid(&json!({ "id": 42 })));
```

If a schema document sets `$schema`, it must be exactly Draft 2020-12
(`https://json-schema.org/draft/2020-12/schema`, with an optional trailing
`#`). Omitting `$schema` is allowed and is interpreted as Draft 2020-12.

## License

Licensed under MIT. See [LICENSE](../LICENSE).

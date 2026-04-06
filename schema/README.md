# json_schema_ast

Strict Draft 2020-12 JSON Schema documents, validation, and resolved schema IR.

[![crates.io](https://img.shields.io/crates/v/json_schema_ast)](https://crates.io/crates/json_schema_ast) [![docs.rs](https://docs.rs/json_schema_ast/badge.svg)](https://docs.rs/json_schema_ast) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
json_schema_ast = "0.3.1"
```

## Usage

```rust
use json_schema_ast::{SchemaDocument, compile};
use serde_json::json;

let raw = json!({
    "type": "object",
    "properties": {
        "id": { "type": "integer" },
        "name": { "type": "string" }
    },
    "required": ["id"]
});

// Build a document. Construction canonicalizes the schema and checks keyword shapes.
let schema = SchemaDocument::from_json(&raw).unwrap();

// Validate instances with the raw-schema validator backend.
assert!(schema.is_valid(&json!({ "id": 42 })).unwrap());
assert!(!schema.is_valid(&json!({ "name": "Ada" })).unwrap());

// Compile a validator directly if you need to own the backend type.
let validator = compile(&raw).unwrap();
assert!(validator.is_valid(&json!({ "id": 42 })));
```

## Public Interface

For validation callers:

- `SchemaDocument::from_json(&Value)` builds a document from a raw Draft 2020-12 JSON Schema.
- `SchemaDocument::is_valid(&Value)` validates instances against the original raw schema.
- `SchemaDocument::canonical_schema_json()` exposes the canonicalized schema used for IR construction and debugging.
- `compile(&Value)` returns the underlying `jsonschema::JSONSchema` validator after this crate's dialect checks.
- `AstError`, `SchemaBuildError`, `SchemaError`, and `CompileError` are the typed error surfaces.

For resolved-IR consumers:

- `SchemaDocument::root()` returns the lazily resolved immutable `SchemaNode` graph.
- `SchemaNode::kind()` exposes the non-exhaustive `SchemaNodeKind` IR for downstream analyzers.
- `SchemaNode::id()` exposes opaque node identity for cycle guards.
- `SchemaNode::accepts_value()` is a low-level evaluator for resolved subgraphs; use `SchemaDocument::is_valid()` for user-visible validation.
- `json_values_equal(&Value, &Value)` compares JSON values using JSON Schema's numeric equality rule.

The supporting constraint types exposed through `SchemaNodeKind` are deliberately
structured: `IntegerBounds`, `NumberBounds`, and `CountRange` reject empty
intervals at construction; `ContainsConstraint` keeps the `contains` schema and
match-count range together; and `PatternConstraint` records whether the internal
Rust matcher can evaluate a JSON Schema pattern.

The resolved IR entrypoints are public because `jsoncompat` and
`json_schema_fuzz` are separate crates; most users only need the validation API.

If a schema document sets `$schema`, it must be exactly Draft 2020-12
(`https://json-schema.org/draft/2020-12/schema`, with an optional trailing
`#`). Omitting `$schema` is allowed and is interpreted as Draft 2020-12.

Same-document refs to `"#"` and `"#/..."` are supported, including recursive
graphs. Pure alias cycles, remote refs, plain-name fragments, and dynamic refs
are rejected with typed resolver errors.

## License

Licensed under MIT. See [LICENSE](../LICENSE).

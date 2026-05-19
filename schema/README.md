# json_schema_ast

Build, validate, and resolve Draft 2020-12 JSON Schema and OpenAPI 3.1 Schema Object documents.

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

let schema = SchemaDocument::from_json(&raw).unwrap();

assert!(schema.is_valid(&json!({ "id": 42 })).unwrap());
assert!(!schema.is_valid(&json!({ "name": "Ada" })).unwrap());

let validator = compile(&raw).unwrap();
assert!(validator.is_valid(&json!({ "id": 42 })));
```

## API

Most callers only need:

- `SchemaDocument::from_json(&Value)` builds a document from a raw Draft 2020-12 JSON Schema or OpenAPI 3.1 Schema Object.
- `SchemaDocument::is_valid(&Value)` validates instances against the original raw schema.
- `compile(&Value)` returns a ready-to-use validator after this crate's dialect checks.

Lower-level APIs for canonicalized schema access, resolved graph traversal, and typed errors are documented on [docs.rs](https://docs.rs/json_schema_ast). [developing.md](../developing.md) explains how this repository uses them internally.

If a schema document sets `$schema`, it must be either Draft 2020-12
(`https://json-schema.org/draft/2020-12/schema`, with an optional trailing
`#`) or the OpenAPI 3.1 Schema Object dialect
(`https://spec.openapis.org/oas/3.1/dialect/base`). Omitting `$schema` is
allowed and is interpreted as Draft 2020-12. OpenAPI 3.0-only schema semantics
such as `nullable` are not interpreted; use the OpenAPI 3.1 / JSON Schema form
instead.

Same-document refs to `"#"` and `"#/..."` are supported, including recursive
graphs. Pure alias cycles, remote refs, plain-name fragments, and dynamic refs
are rejected with typed resolver errors.

## More detail

- [Developer guide](../developing.md) for resolved-IR internals and constraint design
- [Repository README](../readme.md) for the broader `jsoncompat` workflow
- [docs.rs](https://docs.rs/json_schema_ast) for API reference

## License

Licensed under MIT. See [LICENSE](../LICENSE).

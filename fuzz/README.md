# json_schema_fuzz

Schema-guided JSON value generation for Draft 2020-12 JSON Schema documents.

[![crates.io](https://img.shields.io/crates/v/json_schema_fuzz)](https://crates.io/crates/json_schema_fuzz) [![docs.rs](https://docs.rs/json_schema_fuzz/badge.svg)](https://docs.rs/json_schema_fuzz) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
json_schema_fuzz = "0.3.0"
```

## Usage

```rust
use json_schema_ast::SchemaDocument;
use json_schema_fuzz::{GenerationConfig, ValueGenerator};
use serde_json::json;

let raw = json!({
    "type": "object",
    "properties": {
        "flag": { "type": "boolean" },
        "count": { "type": "integer" }
    },
    "required": ["flag"]
});

// Keep the schema document around; it lazily builds the canonicalized graph
// and raw validator when generation needs them.
let schema = SchemaDocument::from_json(&raw).unwrap();

// Generate a random value with the default depth and retry budget.
let mut rng = rand::rng();
let value = ValueGenerator::generate(&schema, GenerationConfig::default(), &mut rng).unwrap();

println!("{}", value);
```

For repeated generation from the same schema, keep the same `SchemaDocument` so its lazy
canonical graph and raw validator are reused.

## Public Interface

- `ValueGenerator::generate(&SchemaDocument, GenerationConfig, rng) -> Result<Value, GenerateError>` is the value-generation entry point.
- `GenerationConfig::default()` uses the default recursion depth and retry budget.
- `GenerationConfig::new(depth)` changes the recursion depth limit while keeping the default retry budget.
- `GenerationConfig::with_max_generation_attempts(limit)` overrides the retry budget with a non-zero limit.
- `GenerateError::Unsatisfiable` means the resolved schema is known to have no valid instances.
- `GenerateError::ExhaustedAttempts` means the schema may still be satisfiable, but the heuristic generator did not find a raw-valid candidate within the configured retry budget.

The generator walks the canonicalized `SchemaNode` graph from `json_schema_ast`,
but every returned value is accepted by `SchemaDocument::is_valid()` against the
original raw schema.

## License

Licensed under MIT. See [LICENSE](../LICENSE).

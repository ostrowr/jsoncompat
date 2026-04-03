# json_schema_fuzz

JSON Schema Fuzzer: generate random JSON instances conforming to a JSON Schema.

[![crates.io](https://img.shields.io/crates/v/json_schema_fuzz)](https://crates.io/crates/json_schema_fuzz) [![docs.rs](https://docs.rs/json_schema_fuzz/badge.svg)](https://docs.rs/json_schema_fuzz) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
json_schema_fuzz = "0.2.6"
```

## Usage

```rust
use json_schema_ast::{build_and_resolve_schema, SchemaNode};
use json_schema_fuzz::ValueGenerator;
use serde_json::json;
use rand::thread_rng;

let raw = json!({
    "type": "object",
    "properties": {
        "flag": { "type": "boolean" },
        "count": { "type": "integer" }
    },
    "required": ["flag"]
});

// Build AST
let schema_node: SchemaNode = build_and_resolve_schema(&raw).unwrap();

// Generate a random value
let mut rng = thread_rng();
let mut generator = ValueGenerator::new();
let value = generator.generate_value(&schema_node, &mut rng, 4);

println!("{}", value);
```

If you only need a single sample, `generate_value(&schema_node, &mut rng, depth)` is still
available as a one-shot helper. For repeated generation from the same AST, prefer
`ValueGenerator` so compiled subvalidators are cached across calls.

## License

Licensed under MIT. See [LICENSE](../LICENSE).

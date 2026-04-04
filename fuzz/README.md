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
use json_schema_ast::ResolvedSchema;
use json_schema_fuzz::ValueGenerator;
use rand::thread_rng;
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
let schema = ResolvedSchema::from_json(&raw).unwrap();

// Generate a random value
let mut rng = thread_rng();
let mut generator = ValueGenerator::new();
let value = generator.generate_value(&schema, &mut rng, 4).unwrap();

println!("{}", value);
```

If you only need a single sample, `generate_value(&schema, &mut rng, depth)` is available
as a one-shot helper. For repeated generation from the same schema, keep the same
`ResolvedSchema` so its lazy canonical graph and raw validator are reused.

## License

Licensed under MIT. See [LICENSE](../LICENSE).

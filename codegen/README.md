# json_schema_codegen

Generate language-specific models from JSON Schema. The crate builds an in-memory model graph and
lets generators render code for different languages while preserving the serializer/deserializer
split described in the main project README.

## Usage

```rust
use json_schema_codegen::{generate_pydantic, PydanticOptions};
use serde_json::json;

let schema = json!({
    "type": "object",
    "properties": {
        "id": { "type": "integer" },
        "name": { "type": "string", "default": "Anonymous" }
    },
    "required": ["id"]
});

let code = generate_pydantic(&schema, "User", PydanticOptions::default())?;
println!("{code}");
```

The generated output includes paired `Serializer` and `Deserializer` models. Defaults are applied
only on the deserializer; serializer models override `model_dump` / `model_dump_json` to exclude
unset fields by default.

## Supported JSON Schema (current)

This crate is intentionally strict: unsupported features raise errors rather than generating
inaccurate code.

- Local `$ref` values only (`#` JSON pointers); external refs are rejected.
- `allOf` is supported only for object schemas with non-conflicting properties.
- `anyOf`/`oneOf` are emitted as union types (no exclusivity enforcement for `oneOf`).
- `enum`/`const` must contain JSON primitives (null, boolean, number, string).
- Object schemas support `properties`, `required`, `additionalProperties`, `minProperties`,
  `maxProperties`, `title`, and `description`.
- Array schemas support `items`, `minItems`, `maxItems` (tuple-style `items` is rejected).
- String schemas support `minLength`, `maxLength`, `pattern`, and `format`.

Unsupported features include (but are not limited to) `patternProperties`, `propertyNames`,
`dependentRequired`, `dependentSchemas`, `unevaluatedProperties`, `contains`, `prefixItems`, and
`uniqueItems`.

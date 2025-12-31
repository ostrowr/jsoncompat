# json_schema_codegen

Generate language-specific models from JSON Schema. The crate builds an in-memory model graph and
lets generators render code for different languages while preserving the serializer/deserializer
split described in the main project README.

## Usage

```rust
use json_schema_ast::build_and_resolve_schema;
use json_schema_codegen::{pydantic, ModelRole, PydanticOptions};
use serde_json::json;

let schema = build_and_resolve_schema(&json!({
    "type": "object",
    "properties": {
        "id": { "type": "integer" },
        "name": { "type": "string", "default": "Anonymous" }
    },
    "required": ["id"]
}))?;

let base_module_name = "json_schema_codegen_base";
std::fs::write(
    format!("{base_module_name}.py"),
    json_schema_codegen::pydantic_base_module(),
)?;

let options = PydanticOptions::default()
    .with_root_model_name("User")
    .with_base_module(base_module_name);

let serializer = pydantic::generate_model(&schema, ModelRole::Serializer, options.clone())?;
let deserializer = pydantic::generate_model(&schema, ModelRole::Deserializer, options)?;
println!("{serializer}");
println!("{deserializer}");
```

The contract is `generate_model(schema, role)`: call once for each role you need. Defaults are
applied only on the deserializer; serializer models rely on a shared base module that overrides
`model_dump` / `model_dump_json` to exclude unset fields by default.

The generated Python modules depend on `pydantic>=2` and the Rust-backed `jsonschema-rs` Python
package for runtime validation.

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

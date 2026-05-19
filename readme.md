[![jsoncompat logo](web/jsoncompatdotcom/public/logo192.png)](https://jsoncompat.com)

# jsoncompat

[![crates.io](https://img.shields.io/crates/v/jsoncompat)](https://crates.io/crates/jsoncompat) [![docs.rs](https://docs.rs/jsoncompat/badge.svg)](https://docs.rs/jsoncompat) [![PyPI](https://img.shields.io/pypi/v/jsoncompat.svg)](https://pypi.org/project/jsoncompat/) [![npm](https://img.shields.io/npm/v/jsoncompat.svg)](https://www.npmjs.com/package/jsoncompat) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Check whether evolving JSON Schemas and OpenAPI 3.1 contracts stay backward-compatible.

jsoncompat compares:

- raw JSON Schema Draft 2020-12 documents;
- OpenAPI 3.1 Schema Objects;
- JSON OpenAPI 3.1 documents with path operations.

If a schema declares `$schema`, it must use Draft 2020-12 or the OpenAPI 3.1 Schema Object dialect. OpenAPI 3.0-only shortcuts such as `nullable` are not reinterpreted.

> [!WARNING]
> jsoncompat is alpha software. It is intentionally conservative in places, and it can still miss incompatible changes or report false positives.
>
> The full docs and examples live at [jsoncompat.com](https://jsoncompat.com).

## Install

Install the CLI with Cargo:

```bash
cargo install jsoncompat
```

Python and JavaScript/WebAssembly packages are documented separately:

- [Python bindings](pybindings/README.md)
- [JavaScript/WebAssembly bindings](wasm/README.md)

## Quick start

Check a serializer-facing schema change:

```bash
jsoncompat compat old-schema.json new-schema.json --role serializer
```

Check both serializer and deserializer compatibility, and ask for fuzzed counterexamples when static analysis finds a problem:

```bash
jsoncompat compat old-schema.json new-schema.json --role both --fuzz 1000 --depth 8
```

Check an OpenAPI 3.1 contract:

```bash
jsoncompat compat --openapi old-openapi.json new-openapi.json
```

Generate example values accepted by a schema:

```bash
jsoncompat generate schema.json --count 5 --pretty
```

Compare schema golden files in CI:

```bash
jsoncompat ci old-golden.json new-golden.json --display table
```

Inspect the per-operation request and response schemas generated from an OpenAPI document:

```bash
jsoncompat lower-openapi openapi.json
```

Run the guided CLI demo:

```bash
jsoncompat demo --noninteractive
```

## Stamped schemas

`jsoncompat stamp` turns a schema into separate writer and reader schemas using
a versioned envelope:

```json
{
  "version": 2,
  "data": {
    "name": "Ada"
  }
}
```

Writers emit only the latest schema version, while readers accept a tagged
union of historical writer versions. The command stores schema history in a
manifest file and appends a new version whenever a change is not compatible in
both directions.

```bash
jsoncompat stamp --manifest schemas.manifest.json --id user-profile --write-manifest schema.json
jsoncompat stamp --manifest schemas.manifest.json --id user-profile --display writer schema.json > writer.schema.json
jsoncompat stamp --manifest schemas.manifest.json --id user-profile --display reader schema.json > reader.schema.json
jsoncompat codegen --target schema reader.schema.json
jsoncompat codegen --target dataclasses reader.schema.json > reader_models.py
```

### Dataclass code generation

`jsoncompat codegen --target dataclasses` accepts any JSON Schema document,
canonicalizes it with `SchemaDocument::canonical_schema_json()`, and emits
frozen, slotted Python dataclasses that import shared construction and
serialization helpers from `jsoncompat.codegen.dataclasses`. Generated classes
carry the original input schema in `__jsoncompat_schema__`, cache a
`jsoncompat.validator_for(...)` validator for runtime checks, and expose:

- `from_json(...)` / `from_json_string(...)` constructors for schema-checked
  deserialization;
- `to_json(...)` / `to_json_string(...)` serializers that validate emitted
  JSON against the attached schema;
- `__jsoncompat_extra__` for schema-admitted object properties that are not
  declared under `properties`, including `additionalProperties` and
  `patternProperties`;
- `JSONCOMPAT_MISSING` for omitted optional fields so absent and explicit
  `null` stay distinguishable.

When the schema structure makes it honest, code generation also keeps Python
annotations narrow rather than collapsing to `Any`, including primitive local
`$ref` fields and constrained tuple-like arrays built from `prefixItems`.

If the input schema contains `x-jsoncompat` metadata from `jsoncompat stamp`,
generated writer envelopes inherit from `WriterDataclassModel`, which disables
deserialization methods, and generated reader envelopes inherit from
`ReaderDataclassModel` / `ReaderDataclassRootModel`, which disable
serialization methods.

## Choose a role

Compatibility is directional:

| Role | Question jsoncompat answers |
| --- | --- |
| `serializer` | Can old readers still accept every value the new producer may emit? |
| `deserializer` | Can the new reader still accept every value older producers may have emitted? |
| `both` | Are both directions safe? |

That is why making a previously required response field optional can be breaking for a serializer, while making a previously optional stored field required can be breaking for a deserializer.

## OpenAPI contracts

When the inputs are OpenAPI documents, pass `--openapi`. jsoncompat compares:

- path, query, header, and cookie parameters;
- request bodies and media types;
- response statuses, media types, bodies, and headers;
- removed operations;
- supported local `#/components/...` references.

Requests are checked in the deserializer direction. Responses are checked in the serializer direction. `--role` and `--fuzz` are raw-JSON-Schema-only flags.

See [openapi/README.md](openapi/README.md) for the OpenAPI user guide.

## Rust API

Schema compatibility:

```rust
use jsoncompat::{Role, SchemaDocument, check_compat};
use serde_json::json;

let old = SchemaDocument::from_json(&json!({ "type": "string" })).unwrap();
let new = SchemaDocument::from_json(&json!({ "type": ["string", "null"] })).unwrap();

let compatible = check_compat(&old, &new, Role::Deserializer).unwrap();
```

OpenAPI compatibility:

```rust
use jsoncompat::{OpenApiDocument, check_openapi_compat};
use serde_json::json;

let old = OpenApiDocument::from_json(&json!({
    "openapi": "3.1.0",
    "info": { "title": "Pets", "version": "1.0.0" },
    "paths": {}
})).unwrap();
let new = old.clone();

let report = check_openapi_compat(&old, &new).unwrap();
assert!(report.is_compatible());
```

The Rust API also exposes structured compatibility errors, OpenAPI issue reports, incompatibility explanations, and schema-guided value generation.

## Warnings and hard errors

jsoncompat keeps warnings and hard errors separate:

- unsupported-but-valid schema details produce warnings and the modeled comparison continues;
- inputs that would make a verdict unsafe fail before comparison;
- unsupported OpenAPI contract surfaces fail before comparison rather than being silently ignored.

The CLI prints warnings with exact pointers so you can see what was ignored. See [developing.md](developing.md) for the detailed support boundaries and the reasoning behind them.

## What to read next

- [jsoncompat.com](https://jsoncompat.com) for polished documentation and examples
- [openapi/README.md](openapi/README.md) for OpenAPI-specific usage
- [developing.md](developing.md) for repository layout, internals, tests, fixtures, benchmarks, and release notes
- [docs.rs](https://docs.rs/jsoncompat) for the Rust API reference

## License

MIT License. See [LICENSE](LICENSE).

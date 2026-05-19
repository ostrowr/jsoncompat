# OpenAPI support in jsoncompat

Most users should start with the [repository README](https://github.com/ostrowr/jsoncompat/blob/main/readme.md). This page focuses on how jsoncompat treats OpenAPI 3.1 documents.

jsoncompat can compare JSON OpenAPI 3.1 documents with path operations:

```bash
jsoncompat compat --openapi old-openapi.json new-openapi.json
```

It can also print the per-operation request and response schemas that drive that comparison:

```bash
jsoncompat lower-openapi openapi.json
```

## What gets compared

For supported OpenAPI documents, jsoncompat checks:

- path, query, header, and cookie parameters;
- request bodies, requiredness, media types, and schemas;
- response statuses, media types, bodies, and headers;
- removed operations;
- supported local `#/components/...` references.

OpenAPI compatibility is directional by surface:

- requests are checked in the deserializer direction, because a server should keep accepting requests it previously accepted;
- responses are checked in the serializer direction, because clients should keep accepting responses the server may emit.

`--role` and `--fuzz` are raw-JSON-Schema-only flags. `--openapi` selects the OpenAPI contract path explicitly, and those comparisons always use the paired request/response interpretation above.

## Supported inputs

jsoncompat accepts JSON OpenAPI 3.1 documents. It supports the OpenAPI 3.1 Schema Object dialect and Draft 2020-12 schema dialects already supported by the root library.

Common OpenAPI metadata is allowed when it does not affect the contract being compared. That includes ordinary info, tags, security metadata, descriptions, examples, reference-object metadata, and shape-checked schema annotations such as `readOnly`, `writeOnly`, and `discriminator`.

## Unsupported contract surfaces

jsoncompat fails early when a valid OpenAPI feature could affect compatibility but is not modeled yet. Current examples include:

- document-level `webhooks`;
- path-item `$ref` entries;
- operation callbacks;
- response links;
- unsupported component collections such as callback/path-item/link/example collections;
- media-type `encoding`;
- unsupported remote references;
- content maps whose media keys collapse to the same normalized selector;
- unsupported schema compatibility surfaces inside OpenAPI contracts.

Those inputs fail before a compatibility verdict is reported, rather than being silently ignored.

## Rust API

The root crate exposes the OpenAPI entrypoints:

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

`OpenApiCompatibilityReport::issues()` lists operation removals plus request- and response-surface incompatibilities.

## More detail

- [Repository README](https://github.com/ostrowr/jsoncompat/blob/main/readme.md)
- [Developer guide](https://github.com/ostrowr/jsoncompat/blob/main/developing.md) for validation/lowering internals, test strategy, and unsupported-surface design
- [jsoncompat.com](https://jsoncompat.com)

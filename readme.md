[![jsoncompat logo](web/jsoncompatdotcom/public/logo192.png)](https://jsoncompat.com)

# jsoncompat

[![crates.io](https://img.shields.io/crates/v/jsoncompat)](https://crates.io/crates/jsoncompat) [![docs.rs](https://docs.rs/jsoncompat/badge.svg)](https://docs.rs/jsoncompat) [![PyPI](https://img.shields.io/pypi/v/jsoncompat.svg)](https://pypi.org/project/jsoncompat/) [![npm](https://img.shields.io/npm/v/jsoncompat.svg)](https://www.npmjs.com/package/jsoncompat) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Check whether evolving JSON Schemas and OpenAPI 3.1 contracts stay backward-compatible. Generate typed Python models and HTTP clients from the same schemas.

jsoncompat compares:

- raw JSON Schema Draft 2020-12 documents;
- OpenAPI 3.1 Schema Objects;
- JSON OpenAPI 3.1 documents with path operations.

By default, `$schema` must use Draft 2020-12 or the OpenAPI 3.1 Schema Object dialect. Rust callers can supply Draft 2020-12 vocabulary metaschemas and external resources through `SchemaOptions`. OpenAPI 3.0-only shortcuts such as `nullable` are not reinterpreted.

> [!WARNING]
> jsoncompat is alpha software. It is intentionally conservative: incomplete proofs without validated counterexamples are reported as `Unknown`.
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

## Numeric precision

Arbitrary precision is **off by default**, including in the Python and
WebAssembly builds. Rust crates use the ordinary `serde_json` representation:
signed/unsigned 64-bit integers and finite `f64` values. Decimal input can round,
very small numbers can underflow to zero, and numbers beyond that range can fail
to parse. Validation, compatibility checks, generation, and code generation work
with the represented values; they cannot recover digits lost during parsing.
This trades complete numeric correctness for speed and serialization compatibility.
The validator backend enables `serde_json/float_roundtrip`, so serialized finite
`f64` values parse back to the same value. This does not preserve arbitrary
decimal precision or extend the numeric range.

Enable exact JSON number parsing explicitly:

```toml
jsoncompat = { version = "0.5.0", features = ["arbitrary-precision"] }
```

The same opt-in feature is available on `json_schema_ast`, `json_schema_fuzz`,
`jsoncompat_codegen`, and `jsoncompat_openapi`, and is forwarded to their internal
dependencies. To enable it in the CLI, build with
`cargo install jsoncompat --features arbitrary-precision`.

The feature enables `serde_json/arbitrary_precision` throughout the shared Cargo
dependency graph. It changes number storage and serialization behavior and adds
allocation and parsing costs, even for ordinary numbers. `default-features = false`
cannot disable it if another dependency explicitly enables it. Existing exact
arithmetic remains in use in either mode, but only the enabled mode preserves
arbitrary JSON number text. See the binding READMEs for opt-in build commands.

## Quick start

Check a serializer-facing schema change:

```bash
jsoncompat compat old-schema.json new-schema.json --role serializer
```

Check both serializer and deserializer compatibility, and ask for fuzzed counterexamples when static analysis finds a problem:

```bash
jsoncompat compat old-schema.json new-schema.json --role both --fuzz 1000 --depth 8
```

Use `--json` for a structured verdict. Exit codes are 0 for compatible, 1 for incompatible or invalid input, and 2 for unknown.

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

## Dataclass code generation

Generate frozen, slotted Python models with typed constructors and validation:

```bash
jsoncompat codegen --target dataclasses schema.json --output models.py
```

Ship `models.py` and its generated `_models_generated.py` companion together.
Use `Model.from_value(...)` and `.to_value()` for Python values, or
`Model.deserialize(...)` and `.serialize()` for JSON. Optional fields default to
`MISSING`, keeping omission distinct from an explicit `None`.

Generation prepares validation and conversion programs; importing or first using
a model does not compile its schema. Keep the original schema as your build
input. See the [Python model guide](pybindings/README.md#generated-model-artifacts)
for omitted fields, immutable collections, YAML/MessagePack, and stamped models.

To generate models from OpenAPI components without HTTP requests:

```bash
jsoncompat codegen --openapi openapi.json --output my_models
```

Import directly from a component module, such as
`from my_models.models.order import Order`. See
[OpenAPI model packages](pybindings/README.md#openapi-model-packages).

## Python clients from OpenAPI

Generate typed requests and responses for Python 3.12 or newer:

```bash
jsoncompat codegen --openapi openapi.json --api Petstore --output petstore
pip install 'jsoncompat[requests]'
```

Bind a session you own, then pass a generated request to `client.request()`:

```python
import requests
from jsoncompat.http import bind
from jsoncompat.http.requests import RequestsFetch
from petstore.api import PetstoreAPI
from petstore.operations.pets import GetPetRequest

with requests.Session() as session:
    client = bind(PetstoreAPI, fetch=RequestsFetch(session, "https://api.example.com"))
    response = client.request(GetPetRequest(pet_id="p_123"))
    print(response.status)  # The body has been downloaded, but not parsed.
    pet = response.unwrap()  # Parse and validate; return GetPet200 or raise.
    print(pet.body.name)
```

Replace the URL with your API's base URL. Configure credentials and defaults on
your session; the adapter never creates or closes it. Use `bind_async` with
`AiohttpFetch` or `AsyncHttpxFetch` for async code, or `HttpxFetch` with `bind`.
Install the corresponding `jsoncompat[aiohttp]` or `jsoncompat[httpx]` extra.

- `.status`, `.headers`, and `.content` expose the buffered response without parsing.
- `.unwrap()` returns a typed success or raises. `.result()` gives `Ok`/`Err` for
  handling expected errors such as a 404. Both share one cached decoding result.
- Undeclared or malformed non-2xx responses become `UnexpectedResponse`, preserving
  the status, headers, and body. Invalid successes raise `ResponseError` when parsed.

Pass clients around using the generated protocol aliases:

```python
from petstore.api import PetstoreClient
from petstore.operations.pets import GetPet200, GetPetRequest

def get_pet(client: PetstoreClient, pet_id: str) -> GetPet200:
    return client.request(GetPetRequest(pet_id=pet_id)).unwrap()
```

`PetstoreAsyncClient` is the async equivalent. Importing `PetstoreAPI` binds the
precomputed dispatch plans; `bind` only attaches the fetcher. Requests live in
resource modules such as `operations/pets.py`, and models in `models/pet.py`.
Publish the complete generated directory, including `py.typed`.

Start with the [checked-in Petstore client](examples/openapi_client/petstore/)
and [runnable aiohttp, HTTPX, and requests examples](examples/openapi_client/README.md).
They cover expected errors, unexpected responses, and per-call timeouts.
The [HTTP reference](pybindings/README.md#api-scoped-http-requests) explains custom
fetchers, transport typing, and request overrides. The
[public-spec audit](pybindings/benchmarks/public-openapi.md) records real-world
coverage and current support limits.

## Choose a role

Compatibility is directional:

| Role | Question jsoncompat answers |
| --- | --- |
| `serializer` | Can old readers still accept every value the new producer may emit? |
| `deserializer` | Can the new reader still accept every value older producers may have emitted? |
| `both` | Are both directions safe? |

That is why making a previously required response field optional can be breaking for a serializer, while making a previously optional stored field required can be breaking for a deserializer.

Deserializer checks assume old producers omit optional undeclared object
properties where the checker can establish that assumption safely. Required
properties and dependency-forced properties still have their values checked.
Complex applicator comparisons conservatively use full validation semantics.

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

Use `analyze_compat(&old, &new, role)` for a three-way result: `Compatible`,
`Incompatible { direction, counterexample }`, or `Unknown { reason }`. An
incompatible result always includes a validated witness. The boolean
`check_compat` API returns false when inclusion is not proved, including unknown
cases. The CLI reports that distinction explicitly.

See the [keyword support matrix](keyword-support.md) for parsing, validation,
proof, generation, resource-resolution, and format-vocabulary coverage.

## Unknown results and errors

Malformed schemas, missing resources, unsupported dialects, and unsupported
OpenAPI contract surfaces produce errors. Supported annotations do not produce
warnings or affect compatibility. Complex valid schemas can yield an unknown
proof result. See the [keyword support matrix](keyword-support.md) for precise
boundaries.

## What to read next

- [jsoncompat.com](https://jsoncompat.com) for polished documentation and examples
- [openapi/README.md](openapi/README.md) for OpenAPI-specific usage
- [developing.md](developing.md) for repository layout, internals, tests, fixtures, benchmarks, and release notes
- [docs.rs](https://docs.rs/jsoncompat) for the Rust API reference

## License

MIT License. See [LICENSE](LICENSE).

# jsoncompat

Python bindings for checking compatibility of evolving JSON Schemas and generating example values.

## Installation

Install from PyPI:

```bash
pip install jsoncompat==0.5.0
```

Releases also provide a source distribution for platforms without a matching
wheel. To request a source build explicitly:

```bash
pip install --no-binary=jsoncompat jsoncompat
```

Source builds require Python 3.12 or newer, a current stable Rust toolchain,
and the platform's native linker/build tools. Network access is required to
fetch the build backend, Rust crates, and the pinned Git validator dependency.
The archive includes the local Rust workspace dependencies and Python sources;
you do not need a repository checkout.

## Quick start

```python
import jsoncompat as jsc

old_schema = '{"type": "string"}'
new_schema = '{"type": "number"}'

is_compatible = jsc.check_compat(old_schema, new_schema, jsc.Role.BOTH)
print(is_compatible)

# Generate example values for a schema
generator = jsc.generator_for(old_schema)
example = generator.generate_value(depth=5)
print(example)
```

## API

- `check_compat(old_schema_json: str, new_schema_json: str, role: str = "both") -> bool`
  - `role` must be `"serializer"`, `"deserializer"`, or `"both"`.
  - Raises `ValueError` for invalid schemas or unsupported dialects or unresolved resources.
- `generator_for(schema_json: str) -> Generator`
  - Parses the schema once and returns a reusable generator.
  - `Generator.generate_value(depth: int = 5) -> str` returns a JSON string for one generated value accepted by the schema.
  - Raises `ValueError` when the schema is invalid, known to be unsatisfiable, or generation exhausts its retry budget.
- `validator_for(schema_json: str) -> Validator`
  - Parses the schema once and returns a reusable validator.
  - `Validator.is_valid_json(instance_json: str) -> bool` validates JSON strings against the parsed schema.
  - `Validator.is_valid_value(instance: JsonValue) -> bool` validates Python JSON-compatible values: `None`, `bool`, finite `int`/`float`, `str`, `list`, `tuple`, and `dict[str, ...]`.
- `generate_value(schema_json: str, depth: int = 5) -> str`
  - Deprecated. Use `generator_for(schema_json).generate_value(depth)` instead.
- `is_valid(schema_json: str, instance_json: str) -> bool`
  - Deprecated. Use `validator_for(schema_json).is_valid_json(instance_json)` instead.
- `jsoncompat.codegen.dataclasses` runtime helpers for generated dataclass models
- `Role.SERIALIZER`, `Role.DESERIALIZER`, and `Role.BOTH` are string constants accepted by `check_compat`.

Generated dataclasses use `from_value(...)` / `to_value(...)` for Python JSON
values and `deserialize(...)` / `serialize(...)` for encoded JSON, YAML, and
MessagePack. JSON is the default format. Install optional codecs with
`jsoncompat[yaml]` or `jsoncompat[msgpack]`. All direct constructors and
conversion methods accept keyword-only `skip_validation=True` when the caller
already guarantees schema validity. It skips only the attached JSON Schema
check; wire-format parsing and JSON-value normalization, runtime type
conversion, and reader/writer direction guards still apply.

Only classes emitted by `jsoncompat codegen --target dataclasses` implement
this runtime contract. The generator builds the shared conversion plan and
validation programs ahead of time. Import binds the plan to the generated
classes, and it is then shared across threads. Custom subclasses, construction
hooks, and Python defaults/default factories are outside the model-definition
surface.

Generated array and object fields accept ordinary lists and dictionaries at
construction boundaries, then store them as deeply immutable values exposed as
`Sequence[...]` and `Mapping[...]`. Use `to_value()` when a mutable JSON-value
tree is required by another API.

See the [canonical plain-schema example](../examples/dataclasses/demo.py) for an
ordinary generated model that both serializes and deserializes. The
[canonical stamped-schema example](../examples/stamp/demo.py) covers versioned
writer/reader envelopes and historical schemas.

## Omitted fields

An optional field has type `Omittable[T]`: it contains either a `T` or
`MISSING`. This also applies to `Model(**data)`: optional fields absent from the
mapping become `MISSING`, while omitted required fields raise an error.
Use `is MISSING` or `is not MISSING` to check whether the field was
omitted. These identity comparisons narrow the type in both branches:

```python
from jsoncompat import MISSING
from models import Order

def describe_note(order: Order) -> str:
    if order.note is MISSING:
        return "No note supplied"
    if order.note is None:
        return "Note explicitly cleared"
    return order.note.upper()  # Narrowed to str; no cast needed.
```

`MISSING` is the sole member of `MissingType`, a standard Python enum. It is
also exported from `jsoncompat.codegen.dataclasses`, and preserves its identity
through copying, pickling, and reloading the top-level package. The optional
`is_missing(value)` helper is available from both modules and also narrows both
branches.

`None`, `False`, zero, empty strings, and empty collections are all present
values. Test presence explicitly: `bool(MISSING)` raises `TypeError` with a
reminder to use an identity comparison.

Pass `MISSING` to a constructor or `dataclasses.replace` to omit a
field again. `to_value()` and `serialize()` leave omitted properties out; an
explicit `None` remains JSON `null` when the schema permits it.

When upgrading, replace `JSONCOMPAT_MISSING` and `JsoncompatMissingType` imports
with `MISSING` and `MissingType`, and regenerate models with the new generator.

## Generated model artifacts

Generation produces models ready to import. The Rust generator resolves model
references and types, compiles schema programs and regexes, and computes
conversion optimizations in the same command that emits Python:

```bash
jsoncompat codegen --target dataclasses schema.json --output models.py
```

The public `models.py` imports a private `_models_generated.py` companion at the
top, followed by the root model and readable field declarations. A binding call
after the classes connects them to the companion's constructor signatures,
schemas, and precomputed runtime programs. Imports support both package and
top-level modules.

Regeneration replaces the same two filenames. Generate them during your build
and deploy both files together; the pair is a single build artifact. Each file
is replaced atomically, but the two-file update is not a live-reload protocol.

Without `--output`, the same generator writes a self-contained module to stdout:

```bash
jsoncompat codegen --target dataclasses schema.json > models.py
```

Its public declarations appear first and implementation follows a marked divider.
These are two packaging layouts of one generated format, with identical behavior.
There is no unprepared model format, runtime fallback compiler, or separate Python
preparation command. Regenerate models produced by older jsoncompat versions.
The generator does not require a Python interpreter. Python's ordinary `.pyc`
cache is still interpreter-specific: wheel installation normally builds it;
source-only deployments should precompile it during packaging when cold import
latency matters.

At import, the runtime creates Python classes and compact field descriptions,
loads distinct precomputed programs once per module, checks their format, and
binds actual class/slot addresses. Repeated validation checks and scalar guards
are shared by the generated programs; unaliased fields reuse one name index.
Generated-model import and first use do not resolve type annotations or compile schemas,
regexes, or dataclass methods. Interpreter startup and native-library loading
remain normal process startup costs.

Generated classes retain frozen/slotted dataclass behavior, field metadata,
constructor signatures, equality, hashing, repr, pickle support, and
`dataclasses.replace`. Common frozen-dataclass methods are shared across classes
to reduce generated code and per-model memory. Full `dataclasses.Field` objects
and their metadata are created and cached only when explicitly requested by
reflection or utilities such as `dataclasses.fields`, `asdict`, or `replace`.
Construction, repr, equality, hashing, and JSON I/O do not trigger that work.
The private `__dataclass_fields__` attribute is a mapping rather than a plain
dict. JSON/YAML/MessagePack APIs and writer/reader restrictions are unchanged. Checked serialization validates current
state, including models constructed with `skip_validation=True` or subsequently
modified through `object.__setattr__`.

Where conversion proves the schema constraints, parsing and serialization check
scalar constraints directly and avoid a complete intermediate JSON tree or a
second schema walk. Other schemas use the same precompiled general validator;
ambiguous unions retain schema-based selection. Ordinary self-referential and
mutually recursive models using local `$ref` are supported, with a runtime depth
guard. Embedded resource identifiers, anchors, and `$dynamicRef` scopes resolve
to local graph references during generation. Unavailable resources, the legacy
`$recursiveRef` keyword, asserted formats/custom vocabularies, and regex
backreferences/atomic groups/subroutine calls fail during generation. Unicode word boundaries
and lookarounds use prebuilt operations. Complex regex compositions have an
execution budget; exhausting it rejects validation, including inside negation,
conditionals, or property patterns. Unsupported features fail explicitly;
constraints are never silently dropped or compiled later. Exact decimal bounds
and divisors are prepared ahead of time. JSON input is validated before conversion
to Python floats; if rounding makes a stored value invalid, checked serialization
rejects it. The value API validates the decimal representation of the Python value.

Schema validation failures from generated models raise `jsoncompat.ValidationError`, a `ValueError`
subclass with `kind`, `instance_path`, `schema_path`, and `keyword`. Paths are
RFC 6901 pointers; the schema pointer refers to the linked schema used by the
prepared program. `kind="resource_limit"` distinguishes evaluation budgets from
ordinary constraint failures. Diagnostics traverse the prebuilt program only
on failure; they do not expand the stored schema or compile a validator.

```python
try:
    Model.deserialize(payload)
except jsoncompat.ValidationError as error:
    print(error.instance_path, error.schema_path, error.keyword, error.kind)
```

`cargo test --test python_dataclasses_differential` compares optimized and
forced-general execution against the original schemas using the independent
validator. It covers the full generated fixture corpus, each input/output
operation, mutations, exact decimals, regexes, 200-field models, and large values.
It also generates 128 schema trees and four equivalent forms of each, with
independent grammar-specific membership checks. Set `JSONCOMPAT_SCHEMA_SEED`
and `JSONCOMPAT_SCHEMA_CASES` to reproduce or expand this campaign. Failures keep
artifacts and attempt bounded schema/value shrinking into
`minimized.failure.json` before reporting the original failure.

Benchmark the build cost, fully checked round trips, and fresh-process startup
against the same strict Pydantic peers used by the existing benchmarks:

```bash
just python-bench-codegen
just python-bench-codegen-large
```

The report at `target/python-codegen/benchmark.json` records all timing samples,
versions, payload sizes, build time, and startup boundaries. Execution order
rotates between implementations. `--assert-target` on
`pybindings/bench_dataclasses_codegen.py` requires generated-model elapsed time to be
at most 70% of Pydantic on **both** the small model and the recursive tree.
The large profile writes `target/python-codegen/large/benchmark.json` and enforces
the same target independently for a 1,000-field model, a schema generating 201
classes, and 12,000 records (about 9.6 MB of JSON, including non-ASCII strings).
It also measures model import and the first round trip separately with the
runtime already imported. Both implementations enforce the emitted length,
numeric, required-property, and additional-property constraints; invalid
values at the end of each large payload are checked before timing.
To increase recursive depth, run the standard script with `--depth 7 --fanout 4
--tree-iterations 10 --output target/python-codegen/deep/benchmark.json`.
Pydantic validates input; jsoncompat also validates output. These are measured
workloads, not a speed guarantee for every schema or machine. The test suite
checks generated models against the general schema validator
across every generated fixture, supplied examples, and deterministic mutations.

Measured on macOS 26.6.2 arm64 with Python 3.12.2, Pydantic 2.13.4, and the
release extension (2026-10-08). Every round-trip case uses nine rotating-order
samples and the split public/private artifacts:

| Workload | JSON bytes | Generated (µs) | Pydantic (µs) | Less elapsed time |
| --- | ---: | ---: | ---: | ---: |
| Small nested model | 228 | 2.503 | 4.068 | 38.5% |
| 1,365-node recursive tree | 268,847 | 2,290 | 3,803 | 39.8% |
| 21,845-node recursive tree | 4,350,138 | 37,093 | 64,904 | 42.8% |
| 1,000-field model | 65,391 | 237 | 398 | 40.4% |
| 201 generated classes | 159,352 | 603 | 1,122 | 46.2% |
| 12,000 records with Unicode strings | 9,622,824 | 44,701 | 74,811 | 40.2% |

Both benchmark suites pass `--assert-target`, including the deeper recursive
tree. All six comparison cases take at least 30% less elapsed time than Pydantic.
The native CPython path probes a bounded prefix of long strings. When it
encounters non-ASCII text there, it allocates the final character width and
length once, avoiding repeated widening and resizing. Other strings and Python
runtimes retain the standard constructors. Validation remains enabled.

In a separate alternating before/after comparison, the 9.6 MB round trip fell
from 46.61 ms to 40.55 ms during this optimization pass. Three-sample controls
using the same records with different string contents put the final ASCII-only
round trip within 1% of the previous runtime (14.18 versus 14.29 ms); Latin-1
and BMP controls take 8.9% and 18.0% less time, respectively.

With runtimes preloaded, the 201-class schema imports in
4.71 ms versus 109.79 ms for Pydantic. Its one-step generation and Python
bytecode build takes 217 ms. The small model imports in 0.42 ms versus
1.09 ms; its first round trip takes 68 µs versus 63 µs. Pydantic's common model
machinery is warmed before timing model import, just as the jsoncompat runtime
is preloaded. Full fresh-process import plus first round trip is
42.1 ms versus 103.7 ms. First-call measurements include ordinary allocation and cache
warming, but no deferred schema or model compilation.

To measure a real package containing unique classes and constraints:

```bash
just python-bench-imports 1000 0 2048
just python-bench-imports 200000 5 6000
```

Arguments are class count, fields per class, and retained-memory budget in MiB.
Zero fields selects a mix of 5, 20, 50, 100, and 200. The benchmark builds an
OpenAPI input containing every model, generates bounded modules, and creates a
Pydantic package with the same lazy export policy. It measures fresh interpreters
with and without prebuilt bytecode, package import, first class access and round
trip, population loading, peak RSS, build time, artifact size, and checked round
trips. Filesystem caches are not flushed. The report records the revision,
working-tree state, interpreter, platform, requested population, and actual loaded
population at `target/python-codegen/unique-imports/report.json`.

Runtime workers stop at the memory budget between modules and have a timeout.
Building the full input has its own time and memory costs; choose a population
that fits the build machine. Partial runs are reported without extrapolation.
Earlier cached-shard measurements are not evidence of unique-package startup and
have been removed from this guide. All steady-state benchmarks share
`benchmark_harness.py` for warmup, rotating order, GC policy, and timing samples.

`Model.__jsoncompat_schema__` decompresses and caches the original source only
on explicit access; imports, first round trips, and validation errors do not
expand it. Dataclass reflection similarly materializes full field metadata only
when requested. Generation removes compilation from startup; creating and
connecting runtime objects still costs time and memory.

Schemas are passed as JSON strings. `check_compat` returns a boolean verdict and raises `ValueError` for invalid JSON, invalid schemas, or hard unsupported compatibility cases.


## OpenAPI model packages

```bash
jsoncompat codegen --openapi openapi.json --output my_models --models-per-module 128
python -m compileall -q my_models
```

```python
from my_models import Order, Customer
```

The package exports component models by name. `__init__.pyi` makes those exports
visible to type checkers. Public `models_0000.py` modules contain readable class
and field declarations; private companions contain the prepared implementation.
Package import loads only the export index. Accessing a model loads its module
and binds all models in that module. No model compilation moves to first access.

`--component Order` selects a component and its dependencies; repeat the flag
for several roots. `--rename Order=Purchase` changes a public class name.
Name collisions fail with a diagnostic. Reference-connected models stay together,
including mutually recursive models, so class identities do not depend on import
order. A connected group may exceed `--models-per-module`. Resource identifiers
and anchor references conservatively keep all components in one group to retain
scope. Without `--api`, this command emits component schema models only.

An output manifest fingerprints the compiler, schemas, options, and managed
files. Unchanged groups reuse their artifacts; edited schemas rebuild affected
groups. Regeneration removes obsolete managed files, preserves unrelated files,
and refuses to overwrite unmanaged files. Deploy the complete directory as one
build artifact; generation is not a live-reload or concurrent-writer protocol.

## API-scoped HTTP requests

```sh
jsoncompat codegen --openapi openapi.json --api Petstore --output petstore
```

This adds a nominal `PetstoreAPI` identity, operation request models such as
`GetPetRequest`, response models, and success/error/result aliases to the
generated package. Every request inherits
`Request[PetstoreAPI, ItsSuccess, ItsError]`. One shared client method handles
every operation and preserves its inferred response type:

```python
from typing import Never

from petstore import PetstoreAPI, GetPetRequest, GetPet200
from jsoncompat.http import Client, EncodedRequest, RawResponse, bind

def fetch(
    request: EncodedRequest, /, *, transport: None = None
) -> RawResponse:
    # Adapt your existing session here. Return the status, all header pairs,
    # and body bytes, including declared HTTP error responses.
    ...

client: Client[PetstoreAPI, Never] = bind(PetstoreAPI, fetch=fetch)
pet = client.request(GetPetRequest(pet_id="p_123")).unwrap()  # GetPet200
print(pet.body.name, pet.headers.etag)

def get_pet(client: Client[PetstoreAPI, Never], pet_id: str) -> GetPet200:
    return client.request(GetPetRequest(pet_id=pet_id)).unwrap()
```

`bind_async(PetstoreAPI, fetch=async_fetch)` provides the async equivalent;
use `await client.request(...)`. Both factories require the API class explicitly
and infer the transport option type from the fetcher's signature.
The API type parameter is invariant: clients and requests cannot widen to a
common API base or accept another API's requests. Strict type checking also
rejects `Client` annotations without both type arguments. Each generated API
class is a distinct identity, even when two packages have identical schemas and
class names. At runtime the binder checks exact registered request classes
before encoding or fetching, including when callers bypass static checking.

`Client[A, Options]` and `AsyncClient[A, Options]` are shared structural protocols;
there are no generated operation methods or per-service client protocols.
Sessions, authentication, retries, and cleanup stay outside this interface.
Generated packages include a `py.typed` marker; include it when publishing them.

### Working with responses and results

`request()` returns `ClientResponse[Success, Error]`. The transport buffers and
releases the HTTP response before returning; jsoncompat parses and validates the
body only when you call `.result()` or `.unwrap()`. No second `await`, mode flag,
or response context manager is needed. The caller still owns the session.

```python
from jsoncompat.http import Ok, Err, UnexpectedResponse
from petstore import GetPetRequest, GetPet404

response = client.request(GetPetRequest(pet_id="p_123"))
print(response.status, response.headers.get("ETag"))  # No model parsing.
print(response.headers.get_all("Set-Cookie"))
# response.content contains buffered bytes; response.raw is the original RawResponse.

match response.result():  # GetPetResult; decode and validate now.
    case Ok(value=pet):
        print(pet.body.name, pet.headers.etag)  # GetPet200.
    case Err(error=GetPet404(body=problem)):
        print(problem.message)
    case Err(error=UnexpectedResponse(status=503) as error):
        print(error.text(), error.headers.get("Retry-After"))
    case result:
        result.unwrap()  # Raise for other non-2xx responses, including 304.
```

For the usual success path, write
`pet = client.request(GetPetRequest(pet_id="p_123")).unwrap()` or
`pet = (await client.request(GetPetRequest(pet_id="p_123"))).unwrap()`.
This returns the generated success **envelope**, with its typed `.body`,
`.headers`, `.status`, and `.raw_headers`. Before decoding, `response.headers` is
an immutable `ResponseHeaders` mapping with case-insensitive lookup. Lookup and
`.get()` return the first value; `.get_all()` returns every matching value as a
tuple. `.raw` preserves original header names, ordering, and duplicate pairs.
These raw headers are distinct from the generated, validated header model.

`.result()` returns `Result[Success, Error]`, an alias for
`Ok[Success] | Err[Error | UnexpectedResponse]`. `PreparedRequest.decode(raw)`
returns that Result directly and decodes immediately. Immutable `Ok` and `Err`
wrappers support `isinstance` and Python pattern matching.

Both response methods share one cached decoding outcome, including contract
failures. Repeated calls never reparse, resend, retry, or copy the decoded model;
concurrent calls on one response also decode once. `Ok.unwrap()` returns its
exact `value`. `Err.unwrap()` raises `HTTPStatusError`, whose `error` is the same
object as `Err.error` and whose `response` is the original buffered `RawResponse`.
Exception handlers can use `isinstance` to narrow `error`; they do not infer an
operation-specific error type.

| Response | `.result()` behavior |
| --- | --- |
| Declared, valid 200–299 | `Ok(success_model)` |
| Declared, valid non-2xx, including redirects and 304 | `Err(error_model)` |
| Undeclared non-2xx | `Err(UnexpectedResponse(...))` |
| Non-2xx with invalid JSON, media type, body, or declared headers | `Err(UnexpectedResponse(...))` |
| Undeclared or invalid 2xx | Raise `ResponseError` |

`UnexpectedResponse` exposes `.status`, `.headers`, `.body` (bytes), and the
original `.response`. `.decode_error` retains the `ResponseError` and its original
cause. `.text(encoding="utf-8", errors="replace")` decodes diagnostic text;
`.json()` explicitly parses JSON into `JsonValue`, regardless of Content-Type,
and raises `ValueError` for invalid JSON or UTF-8. Neither helper guesses a model
or discards the original bytes. Invalid error bodies never become instances of
declared models. A declared range or default is decoded before using the fallback.
A failed exact match does not retry a less-specific range or default.

A malformed success remains a contract failure: catch `ResponseError` around
`.result()` or `.unwrap()` and inspect `.response`. The response's raw metadata
and bytes remain accessible before and after the failure. Invalid fetcher output
(such as a non-integer status or a non-bytes body) is also a contract failure,
not an `UnexpectedResponse`. Network errors, timeouts, and cancellation propagate
from `request()` without being wrapped. Lazy parsing does **not** skip downloading
the body; streaming and headers-only requests are separate future capabilities.

The generator exports `GetPetSuccess`, `GetPetError`, and
`GetPetResult = Result[GetPetSuccess, GetPetError]`. `GetPetResponse` remains the
union of declared decoded models, without wrappers. Selection uses exact status,
then range, then `default`. A default model appears in both success and error
aliases if it can match both; `default` is not inherently an error. An unreachable
declared side is `Never`; `Result[S, Never]` still includes unexpected errors.
Multiple legitimate success variants still need narrowing after `unwrap()`.

Regenerate API packages created before the Result interface: request bases carry
`Request[API, Success, Error]`. Custom clients return `ClientResponse[S, E]`;
construct one from `raw` and `prepare(API, request).decode`. Fetchers still return
`RawResponse` and do not need to implement lazy decoding or manage response objects.

### The fetcher contract

To bind to a generated API, implement **one callable** matching `Fetch[Options]`
or `AsyncFetch[Options]`, exported by `jsoncompat.http`. These are the complete
protocols, using Python 3.12 syntax:

```python
from typing import Protocol
from jsoncompat.http import EncodedRequest, RawResponse

class Fetch[Options](Protocol):
    def __call__(
        self, request: EncodedRequest, /, *, transport: Options | None = None
    ) -> RawResponse: ...

class AsyncFetch[Options](Protocol):
    async def __call__(
        self, request: EncodedRequest, /, *, transport: Options | None = None
    ) -> RawResponse: ...
```

A function, closure, or callable instance works; no inheritance or registration
is required. Use `bind(MyAPI, fetch=my_fetch)` or
`bind_async(MyAPI, fetch=my_async_fetch)`. The generated API class supplies the
operation registry; the fetcher does not need to inspect request models or
implement operation methods.

The binder calls `fetch(encoded_request, transport=transport)` exactly once per
valid call, always passing the keyword, including `None` when omitted. The
fetcher must accept that keyword with a default. A fetcher with no per-call
controls declares `transport: None = None` and explicitly annotates its bound
client as `Client[MyAPI, Never]` (or `AsyncClient[MyAPI, Never]`). This supplies
the option type when there is no concrete type to infer from the callback.
A callback accepting only `request` does not satisfy the contract.

The adapter's responsibilities are:

- Join its configured base URL with `request.target`, an already encoded
  origin-relative path and query. Preserve any base URL prefix and existing
  percent escapes; do not re-encode the target or pass it as separate `params`.
- Send `request.method`, `request.headers` (ordered string pairs), and
  `request.body` (`bytes | None`) without serializing again. `None` means an
  omitted body. Preserve repeated header pairs, or reject them explicitly if
  the HTTP library cannot represent them.
- Return `RawResponse(status, headers, body)` with an integer HTTP status,
  all response header pairs including duplicates, and fully buffered bytes
  after HTTP decompression but before JSON/text parsing. Consume and release
  the response according to the HTTP library's lifetime rules.
- Return every HTTP status, including undeclared errors, for the binder to decode; do not call
  `raise_for_status`. Choose a redirect policy deliberately: following redirects
  hides the intermediate responses from the generated contract. Our examples
  disable redirects.
- Own sessions, pools, credentials, timeout defaults, retries, and cleanup.
  Propagate transport errors and async cancellation. The binder does not retry,
  catch a fetcher's `TypeError` to guess its signature, or close sessions.

`request.operation` exposes endpoint, server, and security metadata if the
adapter needs it. The binder does not choose servers or implement authentication.
The adapter interprets `transport=None` as its defaults; otherwise it receives
the exact supplied object. jsoncompat does not copy, inspect, or runtime-validate
transport options. Their type is checked statically, and the adapter owns any
runtime validation. Prefer immutable option values when sharing concurrent calls.

The resulting client has this structural interface (the async version uses
`async def` with the same parameters and result annotation):

```python
from jsoncompat.http import API, ClientResponse, Request, RequestOptions

class Client[A: API, Options](Protocol):
    def request[S, E](
        self,
        request: Request[A, S, E],
        /,
        *,
        options: RequestOptions | None = None,
        transport: Options | None = None,
    ) -> ClientResponse[S, E]: ...
```

Implement that method directly for a custom client or test double. At a boundary
that needs native controls, annotate `Client[PetstoreAPI, MyOptions]`. At a
transport-independent boundary, use `Client[PetstoreAPI, Never]` (or
`AsyncClient[PetstoreAPI, Never]`). Such a function accepts clients with any
concrete option type but can only pass `transport=None` or omit it. The API
parameter stays invariant; the option parameter is inferred as contravariant.
Both parameters are explicit because Python 3.12 does not support defaults in
type parameter lists.

### Generated metadata and dispatch

The generator emits `Annotated` field metadata (`Path`, `Query`, `Header`,
`Cookie`, `Body`) and an operation registry in `api.py`. Each entry pairs a request
class with its response type and HTTP endpoint metadata. `_api.py` holds the
API identity separately so request/response model imports remain acyclic.
Binding validates the API's registered operations and caches immutable dispatch
plans for up to 128 API classes, shared by sync and async clients. Requests use
those prepared plans. Model validation programs remain prepared during code
generation; binding does not compile schemas. Treat published metadata as
immutable after binding. API identities are final, and unregistered request
types and inherited API identities are rejected by the shared binder.

Request and response names derive from `operationId`, or a deterministic
method/path fallback. Collisions fail generation. Requests flatten parameters
into Python fields; location prefixes disambiguate collisions. `--rename`
continues to rename component models. Operation schemas participate in the same
reference graph as components, preserving shared class identity. Inline nested
model classes are also exported from the package. `--component` selects
additional component roots; every operation and its dependencies remain included.

Request constructors, including `RequestModel(**mapping)`, preserve `MISSING`.
Omitted parameters are not sent; an omitted optional body produces `body=None`
and no Content-Type header. A JSON body containing Python `None` produces
`b"null"`. For several request media types, `body` is a union of generated
`OperationBodyApplicationJson`, `OperationBodyTextPlain`, etc. models, each
containing `content_type` and `value`. `request.serialize()` serializes the
logical request object; the HTTP encoder distributes its fields across the wire.

`EncodedRequest` contains the method, an encoded origin-relative `target`, header
pairs, optional body bytes, and its `Endpoint`. Transports choose the base URL
and must preserve the encoded target. Endpoint metadata exposes effective server
URLs/default variables and security requirements; `SECURITY_SCHEMES` exports the
scheme definitions. The binder does not select servers, acquire credentials,
apply authentication, retry, follow redirects, or close sessions. Configure these
in the transport. The [complete examples](../examples/openapi_client/README.md)
show adapters using caller-owned aiohttp, HTTPX, and requests sessions.

### Optional HTTP integrations

Install only the client library you use:

| Install | Import | Fetcher and options |
| --- | --- | --- |
| `pip install 'jsoncompat[aiohttp]'` | `jsoncompat.http.aiohttp` | `AiohttpFetch`, `AiohttpOptions` |
| `pip install 'jsoncompat[httpx]'` | `jsoncompat.http.httpx` | `HttpxFetch`, `AsyncHttpxFetch`, `HttpxOptions` |
| `pip install 'jsoncompat[requests]'` | `jsoncompat.http.requests` | `RequestsFetch`, `RequestsOptions` |

The requests extra includes `types-requests` for static checking. Core imports
and generated packages work without any HTTP client installed. Importing one
integration does not import the others. No adapter creates or closes a session:
pass an existing session and an absolute API base URL (including any path prefix)
to its constructor, then give the fetcher to `bind` or `bind_async`. The caller
configures authentication, TLS, proxies, pooling, hooks, and session cleanup.

```python
import httpx
from jsoncompat.http import bind
from jsoncompat.http.httpx import HttpxFetch
from petstore import PetstoreAPI, GetPetRequest

with httpx.Client(timeout=30) as session:
    client = bind(PetstoreAPI, fetch=HttpxFetch(session, "https://api.example.com/v1"))
    pet = client.request(GetPetRequest(pet_id="p_123")).unwrap()
    print(pet.body.name)
    # session remains available for other native or generated requests here.
```

Calls buffer and release responses, preserving decompressed body bytes and
repeated response headers. Redirects are returned without following them.
Adapters return every HTTP status for typed decoding and do not call
`raise_for_status`; aiohttp's session-level setting is disabled per call.
Caller-configured hooks can still raise, and native I/O errors and cancellation
propagate unchanged. The binder wraps declared responses in `Ok` or `Err`;
`Err.unwrap()` raises `HTTPStatusError` without performing another request.

Aiohttp explicitly enables response decompression and suppresses its automatic
Content-Type header so an omitted body stays omitted. Requests cannot transmit
repeated request header names, so `RequestsFetch` rejects them rather than
silently dropping values. Request defaults set on the session still apply.

Without `transport=`, aiohttp and HTTPX inherit their session timeout.
Requests has no session timeout setting, so `RequestsFetch(..., timeout=10)`
sets a default in seconds; it also accepts `(connect, read)` or `None`.
Per-call options never change these defaults. `timeout=None` disables timeouts
for HTTPX and requests; for aiohttp use `ClientTimeout(total=None)`. These
integrations currently expose only timeout overrides; other per-call transport
policies belong in a custom fetcher implementing the same protocol.

### Per-call overrides and prepared requests

Use `transport=` for adapter-specific settings such as a single-call timeout.
`Options` can be any adapter-defined type. HTTP client dependencies are optional.
For example, using the shipped aiohttp integration:

```python
import aiohttp
from jsoncompat.http import bind_async, RequestOptions
from petstore import PetstoreAPI, GetPetRequest
from jsoncompat.http.aiohttp import AiohttpFetch, AiohttpOptions

async with aiohttp.ClientSession(timeout=aiohttp.ClientTimeout(total=30)) as session:
    client = bind_async(PetstoreAPI, fetch=AiohttpFetch(session, base_url))
    # Inferred: AsyncClient[PetstoreAPI, AiohttpOptions]
    result = await client.request(
        GetPetRequest(pet_id="p_123"),
        transport=AiohttpOptions(timeout=aiohttp.ClientTimeout(total=2)),
        options=RequestOptions(extra_headers={"X-Trace-ID": "trace-123"}),
    )  # ClientResponse[GetPetSuccess, GetPetError]
    # Later calls without transport= still use the session's 30-second timeout.
```

`AiohttpOptions` is a frozen dataclass with one required
`timeout: aiohttp.ClientTimeout` field. A custom fetcher can define richer options
for authentication, routing, retries, tracing, or other native settings without
changing generated models or the binder. `options=` controls the wire request;
`transport=` goes only to the fetcher and is never serialized into headers,
query parameters, or a body. Both work together while preserving response
inference and API checks. Passing another adapter's options is a type error.

See the [runnable aiohttp, requests, and HTTPX examples](../examples/openapi_client/README.md#per-call-transport-options)
for calls using each integration's native timeout types.

Use `RequestOptions` for fields missing from a spec or an exceptional call:

```python
from jsoncompat.http import RequestOptions

result = client.request(
    GetPetRequest(pet_id="p_123", include_history=False),
    options=RequestOptions(
        extra_headers={"X-Trace-ID": "trace-123"},
        remove_headers=("Accept",),
        remove_query=("includeHistory",),
        extra_query=(("includeHistory", "true"), ("preview", "beta")),
    ),
)
```

The same keyword works on async clients. Header/query inputs accept mappings
or sequences of string pairs; sequences retain duplicates and order. Options
snapshot their inputs into immutable tuples, and affect only this call:

- Header removal and replacement match names case-insensitively. Extra headers
  replace all generated occurrences of the same name and are appended after
  removal, so an explicit extra wins over removal of that name.
- Query removals match case-sensitive, decoded wire names, including expanded
  names such as `filter[species]`. Extra pairs append after removal, with names
  and values percent-encoded. Remove a name and add new pairs to replace it.
  Existing query segments retain their exact encoding.
- Overrides apply after schema validation and serialization. They deliberately
  permit values outside the request schema. They cannot supply missing required
  model fields or bypass API registration checks.
- Removals affect the generated request. Session defaults, authentication, and
  automatically added headers are controlled by the fetcher and may add them back.

For further customization, `RequestOptions(transform=...)` takes a synchronous
`Callable[[EncodedRequest], EncodedRequest]`. It runs once, after the overrides
and before fetching. Use `dataclasses.replace` to change the method, encoded
origin-relative target, headers, or buffered body. The result is checked for
valid wire structure and portable ASCII headers; it must retain the original
`operation` metadata. The selected response contract remains unchanged. Transform
exceptions propagate unchanged. Authentication requiring a final absolute URL
or asynchronous credential refresh belongs in the fetcher.

`prepare` exposes encoding and decoding independently of transport I/O:

```python
from jsoncompat.http import prepare

call = prepare(
    PetstoreAPI,
    GetPetRequest(pet_id="p_123"),
    options=RequestOptions(extra_headers={"X-Trace-ID": "trace-123"}),
)  # PreparedRequest[PetstoreAPI, GetPetResult]

# Your fetcher defines its own typed transport options.
raw = await my_fetch(call.request, timeout=5, follow_redirects=False)
result = call.decode(raw)  # GetPetResult
pet = result.unwrap()  # GetPet200
```

Preparation is synchronous and performs no I/O. The immutable result contains
the explicit API identity (`call.api`), encoded request (`call.request`), and
the selected operation's validated decoder. A caller may inspect `RawResponse`
without decoding, or call `decode` later; only decoding promises the typed
response. Invalid or undeclared 2xx responses raise `ResponseError`, retaining
the raw response; non-2xx decoding failures become `Err(UnexpectedResponse(...))`.
Both bound clients use this same preparation/decoding path.

See [prepared_client.py](../examples/openapi_client/prepared_client.py) for a
runnable example with native HTTPX timeout settings. Timeouts, authentication,
redirects, routing, retries, tracing, and cleanup remain caller-owned. These
interfaces use buffered bytes; streaming uploads, downloads, and SSE require
separate transport/lifetime abstractions and are not enabled by a request option.

### Response validation and supported encodings

Response models contain `status`, a typed `body`, typed `headers`, and
`raw_headers` preserving repeated header values. Responses with content also have
`content_type`; responses declaring no content have `body=None`. Several media
representations produce distinct response variants. Explicit statuses take
precedence over ranges, then `default`. Exact statuses have literal Python types;
range/default statuses remain integers with generated runtime constraints.
Use `result.unwrap()` for success-only inference, or match `Ok`/`Err` and inspect
its generated model. Multiple success media/status variants can still require
`isinstance` checks within the success union.

Declared non-2xx responses such as 304 and 404 decode as `Err`. Undeclared
statuses/media, malformed bodies, missing required response headers, and schema
failures become `Err(UnexpectedResponse(...))` for non-2xx responses. For 2xx
responses they raise `ResponseError`, retaining the `RawResponse` and original
cause. Both behaviors occur only when decoding is requested.
Invalid API metadata raises `ContractError` at binding. Invalid request models
or unrepresentable parameters fail before fetching. Transport exceptions and async
cancellation propagate unchanged. Do not call a transport's `raise_for_status`
before returning its `RawResponse`.
`RawResponse.body` contains bytes after HTTP transport decoding (including
decompression), before JSON or text parsing.

Supported bodies are buffered JSON (`application/json`, concrete
`application/*+json`) and UTF-8 `text/plain` with string schemas. Media parameters
may specify a UTF-8 charset. Response selection ignores charset/case; other media
parameters in the spec are rejected. Headers use case-insensitive names.
Schema-style response headers support strings, integers, numbers, booleans,
and arrays of one of those types; JSON-content headers also support structured
values. Scalar headers reject repetitions, arrays combine repeated lines, and
`Set-Cookie` arrays retain each whole line. Undeclared headers remain in
`raw_headers`.

Path styles `simple`, `label`, and `matrix`, query `form`, `deepObject`,
`spaceDelimited`/`pipeDelimited` arrays with `explode=false`, header `simple`,
and cookie `form` are supported. Schema-style parameters must have non-null
scalar or flat array/object values; nested or null values need JSON `content`.
Booleans use `true`/`false`, numbers use JSON syntax, and URL data is percent
encoded separately from structural separators. Empty exploded query arrays or
objects produce no query pairs. Dot segments in an expanded path and invalid
HTTP header names/values are rejected before network I/O. Request header values
use ASCII for portability across transports; JSON-content headers escape Unicode.
Adapters decode raw response header bytes as Latin-1 without losing repeated values.

Generation rejects unsupported features rather than dropping operations:
multipart/form bodies, binary or streaming transports, wildcard media,
`allowReserved`, `allowEmptyValue`, and the existing unsupported OpenAPI lowering
surfaces (including callbacks, webhooks, path-item references, response links,
and remote references). `readOnly`/`writeOnly` retain the existing JSON Schema
annotation behavior; the generator does not manufacture directional schema
variants. Applications with these requirements should adapt the spec explicitly.

## Offline resources and format policy

Rust `SchemaOptions` is also available through Python's `options=` keyword:

```python
validator = jsoncompat.validator_for(
    '{"$ref":"https://example.com/name"}',
    options={"resources": {"https://example.com/name": {"type":"string"}}},
)
```

The same options apply to `generator_for`, `analyze_compat`, `check_compat`,
`generate_value`, and `is_valid`. `assert_formats` overrides the schema's format
vocabulary policy. Resources are explicit and offline. The CLI accepts their
JSON representation in `--schema-options options.json` for raw-schema
compatibility and both code-generation modes. Unsupported asserted formats in
generated models fail at build time rather than losing validation.

## More detail

- [Basic demo](https://github.com/ostrowr/jsoncompat/blob/main/examples/python/basic/demo.py)
- https://jsoncompat.com
- [Repository README](https://github.com/ostrowr/jsoncompat/blob/main/readme.md)
- [Developer guide](https://github.com/ostrowr/jsoncompat/blob/main/developing.md)

## Benchmarks

Run the generated dataclass runtime microbenchmark from the repository root:

```bash
just python-bench
```

The benchmark pins Pydantic v2 and compares the same valid nested payload using
strict, frozen Pydantic models. Pydantic's dump methods do not revalidate the
model, so their closest jsoncompat comparison is the `skip_validation=True`
serialization path.

To measure fresh-interpreter startup separately from steady-state conversion,
including generated-module import, first trusted use, first checked use, and an
equivalent Pydantic v2 graph:

```bash
just python-bench-startup
```

Each sample runs in a new Python process and reports both absolute time and the
increment over an empty process. Results are written under
`target/python-benchmark` alongside the other repeatable benchmark profiles.

For a larger workload, benchmark a balanced recursive graph containing
discriminated unions, lists, mappings, and omitted fields:

```bash
just python-bench-scale
```

The default depth and fanout produce 1,365 model nodes. Override them, along
with the iteration and repeat counts, as positional recipe arguments. The
underlying script also accepts `--profile` to print cumulative Python profiles
for checked and trusted JSON deserialization.

To benchmark JSON -> generated dataclass -> JSON across the complete checked-in
JSON Schema fixture corpus, and compare Pydantic v2 wherever its generated peer
passes the semantic-equivalence screen:

```bash
just python-bench-fixtures
```

This includes every embedded fuzz schema and both sides of every backcompat
fixture. Jsoncompat timings cover every fixture with a representable schema and
valid sample independently of Pydantic coverage. Generated Pydantic modules
and the detailed JSON report are written to
`target/python-fixture-benchmark`. The report retains explicit entries for
jsoncompat or Pydantic generation failures, semantic mismatches, and schemas
without a shared valid value; those cases are never silently removed from the
denominator. Before timing, generated Pydantic validators are screened against
the jsoncompat schema validator with every fixture test plus deterministic type
and property mutations. This prevents ignored JSON Schema keywords from looking
like performance wins. Pydantic uses one precompiled `TypeAdapter` per generated
type, as recommended for repeated validation. Shared generated values are
checked in separately from the generated model artifacts so fresh clones
benchmark the same values.

## License

MIT License. See:

- https://github.com/ostrowr/jsoncompat/blob/main/LICENSE

`analyze_compat(old_schema_json, new_schema_json, role="both")` returns a typed Python dictionary
with `status`: `compatible`, `incompatible`, or `unknown`. Incompatible
results include `direction` and `counterexample`; unknown results include
`reason`. `check_compat` remains boolean and returns false when no inclusion
proof is available. See the [keyword support matrix](../keyword-support.md).

Incompatible results also include `counterexample_json`, the exact JSON spelling of the witness. Use it when decimal precision exceeds the host language’s number representation.

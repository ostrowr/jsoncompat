# API-scoped requests with caller-owned HTTP clients

This example generates a publishable Python contract from a ten-operation
OpenAPI 3.1 Petstore. The generated package contains validated request/response
models and a nominal `PetstoreAPI` identity. `jsoncompat.http.bind` and
`bind_async` provide one shared `client.request(request)` method. Every request
carries its API and response types, preserving inference and rejecting cross-API
calls. Each transport implements one fetch function; sessions, pools, timeouts,
credentials, retries, and cleanup remain application decisions.

The complete [fetcher contract](../../pybindings/README.md#the-fetcher-contract)
documents exactly what `bind` requires. A sync adapter satisfies `Fetch[Options]`
by implementing
`__call__(self, request: EncodedRequest, /, *, transport: Options | None = None) -> RawResponse`;
an async adapter satisfies `AsyncFetch[Options]` with `async def` instead. Plain
functions work too. The binder always passes `transport` by keyword, including
`None`, which selects adapter defaults. The adapter returns every HTTP status,
body bytes, and all header pairs for decoding. No generated operation methods
or protocol inheritance are needed.

From the repository root, generate the package:

```sh
cargo run --bin jsoncompat -- codegen --openapi examples/openapi_client/openapi.json \
  --api Petstore --output examples/openapi_client/petstore
```

Then run any example. Each starts and stops its own local HTTP server; no
account, external service, or real credential is needed.

```sh
uv run --project pybindings --extra aiohttp python examples/openapi_client/aiohttp_client.py
uv run --project pybindings --extra httpx python examples/openapi_client/httpx_client.py
uv run --project pybindings --extra requests python examples/openapi_client/requests_client.py
uv run --project pybindings --extra httpx python examples/openapi_client/prepared_client.py
```

The HTTPX example exercises both synchronous and asynchronous clients.
Generated files are ignored here: regenerate them from `openapi.json`. To
publish a contract, include the entire generated directory and depend on a
jsoncompat version supporting this HTTP runtime.

```python
import httpx
from petstore import PetstoreAPI, GetPetRequest, GetPet200, GetPet404
from jsoncompat.http import bind
from jsoncompat.http.httpx import HttpxFetch

with httpx.Client(headers={"Authorization": "Bearer example-token"}) as session:
    client = bind(PetstoreAPI, fetch=HttpxFetch(session, "http://127.0.0.1:8000"))
    request = GetPetRequest(pet_id="p_123")
    result = client.request(request)
    if isinstance(result, GetPet200):
        print(result.body.name, result.headers.etag)
    elif isinstance(result, GetPet404):
        print(result.body.message)
```

Run `server.py` separately to use the snippet with a persistent server; it
prints its dynamically assigned URL. Substitute that URL in the snippet.
The runnable examples above handle this automatically.

## Per-call transport options

The adapters ship as [optional integrations](../../pybindings/README.md#optional-http-integrations).
Install `jsoncompat[aiohttp]`, `jsoncompat[httpx]`, or `jsoncompat[requests]`
and import only the integration you use. Each option type is a frozen dataclass
with one required `timeout` field. The binder infers that type from the fetcher's
signature and forwards the exact value to the adapter.

With an existing session and base URL, one call can override the timeout:

```python
# aiohttp: ClientTimeout supports total/connect/socket timeouts.
import aiohttp
from jsoncompat.http import bind_async
from petstore import PetstoreAPI, GetPetRequest
from jsoncompat.http.aiohttp import AiohttpFetch, AiohttpOptions

client = bind_async(PetstoreAPI, fetch=AiohttpFetch(session, base_url))
# AsyncClient[PetstoreAPI, AiohttpOptions]
result = await client.request(
    GetPetRequest(pet_id="p_123"),
    transport=AiohttpOptions(timeout=aiohttp.ClientTimeout(total=2)),
)
```

```python
# requests: a scalar or a (connect, read) timeout in seconds.
from jsoncompat.http import bind
from petstore import PetstoreAPI, GetPetRequest
from jsoncompat.http.requests import RequestsFetch, RequestsOptions

client = bind(PetstoreAPI, fetch=RequestsFetch(session, base_url, timeout=10))
# Client[PetstoreAPI, RequestsOptions]
result = client.request(
    GetPetRequest(pet_id="p_123"),
    transport=RequestsOptions(timeout=(1, 2)),
)
```

```python
# HTTPX: a Timeout object or a scalar in seconds.
import httpx
from jsoncompat.http import bind
from petstore import PetstoreAPI, GetPetRequest
from jsoncompat.http.httpx import HttpxFetch, HttpxOptions

client = bind(PetstoreAPI, fetch=HttpxFetch(session, base_url))
# Client[PetstoreAPI, HttpxOptions]
result = client.request(
    GetPetRequest(pet_id="p_123"),
    transport=HttpxOptions(timeout=httpx.Timeout(2, connect=1)),
)
```

For async HTTPX, use `AsyncHttpxFetch`, `bind_async`, and `await`; it shares
`HttpxOptions`. All of these calls infer `GetPetResponse`. Passing
`RequestsOptions` to the aiohttp client, for example, is a static type error.

Omitting `transport` or passing `None` uses the existing aiohttp/HTTPX session's
timeout or the requests adapter's configured default. Passing
`HttpxOptions(timeout=None)` or `RequestsOptions(timeout=None)` disables the
timeout for that call; aiohttp uses `ClientTimeout(total=None)`. The adapters
do not mutate session settings. Timeout units and meanings come from the
[aiohttp](https://docs.aiohttp.org/en/stable/client_reference.html#aiohttp.ClientTimeout),
[requests](https://requests.readthedocs.io/en/latest/user/advanced/#timeouts),
and [HTTPX](https://www.python-httpx.org/advanced/timeouts/) libraries; a requests
read timeout is not a total request deadline.

[`aiohttp_client.py`](aiohttp_client.py), [`requests_client.py`](requests_client.py),
and [`httpx_client.py`](httpx_client.py) are complete runnable examples. Each
performs a typed call with a native timeout override, then passes the same client
to the shared ten-operation workflow. That workflow explicitly accepts
`Client[PetstoreAPI, Never]` or `AsyncClient[PetstoreAPI, Never]`, importing
`Never` from `typing`. This keeps its boundary independent of the adapter while
retaining the exact API and response types. Functions needing native settings
can instead declare, for example, `AsyncClient[PetstoreAPI, AiohttpOptions]`.
Python 3.12 type parameters have no defaults, so both arguments are explicit.

Add other controls to your own adapter's option type as needed. The binder
performs no runtime inspection or validation of transport options; the adapter
owns those semantics. Sessions, pooling, credentials, retries, and lifetimes
remain caller-owned.

## Wire overrides and prepared requests

The sync and async clients accept `options=RequestOptions(...)`. Use
`extra_headers` to replace headers case-insensitively, `remove_headers` to remove
generated headers, `extra_query` to append query pairs, and `remove_query` to
remove existing names before appending. Inputs are snapshotted, duplicates are
preserved, and changes do not leak into later calls. Removals do not suppress
headers added subsequently by a session or authentication handler.
You can pass `options=` and `transport=` together; only `options` changes the
encoded request, and the transport option object is never serialized.

For unusual wire changes, `transform` accepts a synchronous function from
`EncodedRequest` to `EncodedRequest`, applied after those overrides. API identity
checks, request validation, and the original response decoder remain in place.

[`prepared_client.py`](prepared_client.py) demonstrates the lower-level API:
`prepare(PetstoreAPI, request, options=...)` returns an immutable typed call,
`call.request` is sent through a caller-owned fetcher with native HTTPX timeout
settings using the shipped `HttpxFetch`, and `call.decode(raw)` validates the
response. Preparation performs no network I/O; the same call works with a
synchronous or asynchronous fetcher.
Raw responses can also be inspected without decoding. See the
[full contract](../../pybindings/README.md#per-call-overrides-and-prepared-requests).

## What the spec exercises

- CRUD, pagination, a JSON search endpoint, notes with JSON/text representations,
  and a health endpoint with explicit, ranged, and default responses.
- Shared component models and local parameter/response references.
- Required path fields, simple/label/matrix paths, repeated query arrays,
  `deepObject` filters, pipe-delimited arrays, and JSON-content parameters.
- Request headers and cookies, typed response headers, repeated raw response
  headers, conditional requests, 204 and 304 responses.
- Optional bodies and fields using `MISSING`, nullable JSON values, body media
  variants, validation failures, and declared 400/404/412 errors as values.
- Inherited security/server metadata, with credentials supplied by the transport.

The shipped adapters buffer decompressed response bytes, retain repeated headers,
and release each response while keeping the caller's session open. They do not
call `raise_for_status`; aiohttp's session-level setting is overridden per call.
Caller-configured hooks may still raise. Redirects are disabled so declared
redirects remain visible to the contract; use a custom fetcher for a different
redirect policy. Requests cannot send repeated request header names, so its
adapter rejects those explicitly.
The aiohttp adapter disables automatic Content-Type headers so an omitted body
stays omitted. Raw response header bytes are decoded with Latin-1 in every
adapter. The example session configurations also disable environment proxies where
applicable, keeping the local demo independent of machine proxy settings.

The binder currently buffers bodies. JSON and UTF-8 `text/plain` are supported;
multipart, streaming, binary bodies, wildcard media, `allowReserved`, and
`allowEmptyValue` are rejected during generation. Schema-style parameters
support non-null scalars and flat arrays/objects; JSON-content parameters support
nested and nullable values. See the [runtime contract](../../pybindings/README.md#api-scoped-http-requests)
for details and error behavior.

Validation: `cargo test --test openapi_client` generates fresh packages, checks
static typing and wire serialization, and runs all three examples over loopback.
Socket tests also hold a response open and confirm that each adapter's per-call
timeout raises its native exception while leaving the session usable.
The source-distribution check installs the built wheel into four clean environments:
core without HTTP libraries, then each HTTP extra separately with a socket test.

## Live JSONPlaceholder smoke test

Run all four transports against the public HTTPS API:

```sh
just python-http-live
```

This generates a fresh `JsonPlaceholderAPI` request/response package in a
temporary directory, then runs aiohttp, HTTPX sync/async, and requests through
the same six checks (24 requests total):

- Read `/posts/1` and validate its typed response.
- Filter `/posts?userId=1` and validate the returned posts.
- Create a post with a Unicode title and check the simulated 201 response.
- PATCH only its title; verify `MISSING` fields are omitted and existing fields
  survive in the response.
- Read the original post again to verify the patch did not persist.
- Read a nonexistent post and receive a typed 404 with an empty JSON object.

Each transport passes its native timeout option on the first read and also
exercises ordinary calls without transport options.

[`jsonplaceholder.json`](jsonplaceholder.json) is our small OpenAPI 3.1 contract
for the [documented posts API](https://jsonplaceholder.typicode.com/#guide),
not an upstream-published spec. The service simulates writes without storing
them. Tests use no credentials, retain TLS verification, honor environment proxy
settings, set explicit timeouts, and fail on network or contract errors without
retrying or substituting local responses.

The internet test is ignored by default and is not part of `just check`.
Normal tests still generate its contract and strictly type-check the live test
and adapters. To inspect the generated package or run an individual transport:

```sh
cargo run --locked --bin jsoncompat -- codegen \
  --openapi examples/openapi_client/jsonplaceholder.json \
  --api JsonPlaceholder --output examples/openapi_client/jsonplaceholder
uv run --project pybindings --all-extras --locked python \
  examples/openapi_client/live_jsonplaceholder.py LiveAsyncTests.test_aiohttp
```

Omit the test name to run all four; the other names are
`LiveAsyncTests.test_httpx`, `LiveSyncTests.test_httpx`, and `LiveSyncTests.test_requests`.

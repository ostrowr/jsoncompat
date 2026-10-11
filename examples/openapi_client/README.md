# API-scoped requests with caller-owned HTTP clients

This example generates a publishable Python contract from a ten-operation
OpenAPI 3.1 Petstore. The generated package contains validated request/response
models and a nominal `PetstoreAPI` identity. `jsoncompat.http.bind` and
`bind_async` provide one shared `client.request(request)` method. Every request
carries its API and response types, preserving inference and rejecting cross-API
calls. Each transport implements one fetch function; sessions, pools, timeouts,
credentials, retries, and cleanup remain application decisions.

From the repository root, generate the package:

```sh
cargo run --bin jsoncompat -- codegen --openapi examples/openapi_client/openapi.json \
  --api Petstore --output examples/openapi_client/petstore
```

Then run any example. Each starts and stops its own local HTTP server; no
account, external service, or real credential is needed.

```sh
uv run --project pybindings --group http-examples python examples/openapi_client/aiohttp_client.py
uv run --project pybindings --group http-examples python examples/openapi_client/httpx_client.py
uv run --project pybindings --group http-examples python examples/openapi_client/requests_client.py
uv run --project pybindings --group http-examples python examples/openapi_client/prepared_client.py
```

The HTTPX example exercises both synchronous and asynchronous clients.
Generated files are ignored here: regenerate them from `openapi.json`. To
publish a contract, include the entire generated directory and depend on a
jsoncompat version supporting this HTTP runtime.

```python
import httpx
from petstore import PetstoreAPI, GetPetRequest, GetPet200, GetPet404
from jsoncompat.http import bind
from transports import HttpxFetch

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

## Per-call overrides and transport controls

The sync and async clients accept `options=RequestOptions(...)`. Use
`extra_headers` to replace headers case-insensitively, `remove_headers` to remove
generated headers, `extra_query` to append query pairs, and `remove_query` to
remove existing names before appending. Inputs are snapshotted, duplicates are
preserved, and changes do not leak into later calls. Removals do not suppress
headers added subsequently by a session or authentication handler.

For unusual wire changes, `transform` accepts a synchronous function from
`EncodedRequest` to `EncodedRequest`, applied after those overrides. API identity
checks, request validation, and the original response decoder remain in place.

[`prepared_client.py`](prepared_client.py) demonstrates the lower-level API:
`prepare(PetstoreAPI, request, options=...)` returns an immutable typed call,
`call.request` is sent through a caller-owned fetcher with native HTTPX timeout
settings, and `call.decode(raw)` validates the response. Preparation performs no
network I/O; the same call works with a synchronous or asynchronous fetcher.
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

`transports.py` contains all library-specific code. The adapters preserve raw
response bytes and repeated headers and do not call `raise_for_status`.
Redirects are disabled so declared redirects remain visible to the contract;
applications may choose a different redirect policy. Requests cannot send
repeated request header names, so its adapter rejects those explicitly.
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
uv run --project pybindings --group http-examples --locked python \
  examples/openapi_client/live_jsonplaceholder.py LiveAsyncTests.test_aiohttp
```

Omit the test name to run all four; the other names are
`LiveAsyncTests.test_httpx`, `LiveSyncTests.test_httpx`, and `LiveSyncTests.test_requests`.

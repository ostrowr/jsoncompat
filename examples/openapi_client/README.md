# Typed Python clients from OpenAPI

Generate request and response models, bind your own HTTP session, and call
`client.request(...)`. The same generated package works with aiohttp, HTTPX,
requests, or a fetcher you write yourself.

## Run the examples

From the repository root:

```sh
uv run --project pybindings --extra aiohttp python examples/openapi_client/aiohttp_client.py
uv run --project pybindings --extra httpx python examples/openapi_client/httpx_client.py
uv run --project pybindings --extra requests python examples/openapi_client/requests_client.py
```

Each example starts a local Petstore server and exercises all ten operations.
No account or external service is needed. The HTTPX example runs both sync and
async clients. The complete [generated Petstore client](petstore/) is checked in,
so these examples need no generation step.

The package groups operations into `operations/pets.py`, `operations/groups.py`,
`operations/search.py`, and `operations/health.py`. Each component owns a module
such as `models/pet.py`, including its nested types. Direct resource/component
imports load their dependencies; importing `api.py` prepares the full API for
binding. Publish the whole generated directory.

Start with [the Pet model](petstore/models/pet.py),
[pet requests and responses](petstore/operations/pets.py), and
[the API and client types](petstore/api.py). These files are also goldens: the
test suite regenerates the package and compares every Python source, type stub,
and `py.typed` file byte for byte, including added or removed files.

After changing the spec or generator, refresh them with:

```sh
just regen-openapi-client
```

The equivalent generation command for your own package is:

```sh
cargo run --bin jsoncompat -- codegen \
  --openapi examples/openapi_client/openapi.json \
  --api Petstore --output examples/openapi_client/petstore
```

The build manifest is ignored because it includes the local compiler fingerprint;
it is not needed to import or publish the generated client.

## An aiohttp client

Importing `PetstoreAPI` prepares the API once, before creating a session or
serving traffic. Binding later just attaches the fetcher. Your application owns
the session, credentials, connection pool, and cleanup.

```python
import asyncio
import aiohttp
from jsoncompat.http import bind_async
from jsoncompat.http.aiohttp import AiohttpFetch
from petstore.api import PetstoreAPI, PetstoreAsyncClient
from petstore.operations.pets import GetPet200, GetPetRequest
from server import serve

async def get_pet(client: PetstoreAsyncClient, pet_id: str) -> GetPet200:
    return (await client.request(GetPetRequest(pet_id=pet_id))).unwrap()

async def main() -> None:
    with serve() as base_url:
        async with aiohttp.ClientSession(
            headers={"Authorization": "Bearer example-token"},
        ) as session:
            client = bind_async(PetstoreAPI, fetch=AiohttpFetch(session, base_url))
            pet = await get_pet(client, "p_123")
            print(pet.body.name)

asyncio.run(main())
```

`PetstoreAsyncClient` is a protocol alias for passing clients around. The helper
above also accepts async HTTPX clients and correctly typed fakes. For synchronous
code, use `PetstoreClient` and `bind`:

```python
import requests
from jsoncompat.http import bind
from jsoncompat.http.requests import RequestsFetch
from petstore.api import PetstoreAPI
from petstore.operations.pets import GetPetRequest
from server import serve

with serve() as base_url, requests.Session() as session:
    session.headers["Authorization"] = "Bearer example-token"
    client = bind(PetstoreAPI, fetch=RequestsFetch(session, base_url, timeout=10))
    pet = client.request(GetPetRequest(pet_id="p_123")).unwrap()
    print(pet.body.name)
```

For HTTPX, pass `HttpxFetch(session, base_url)` to `bind`, or
`AsyncHttpxFetch(session, base_url)` to `bind_async`. Both come from
`jsoncompat.http.httpx`. See the [complete HTTPX example](httpx_client.py).

## When parsing happens

```python
response = await client.request(GetPetRequest(pet_id="p_123"))
print(response.status, response.headers.get("ETag"))
print(response.content)  # Buffered bytes; no model parsing yet.
pet = response.unwrap()  # Parse and validate now; return GetPet200.
```

- `request()` downloads the body and releases the HTTP response.
- `.status`, `.headers`, `.content`, and `.raw` never parse the body.
- `.unwrap()` and `.result()` parse once and share the cached outcome.

`.unwrap()` returns a typed success response or raises `HTTPStatusError` for an
HTTP error. Invalid successful responses raise `ResponseError` when parsed.
Transport errors occur during `request()` and retain their native exception type.
This is buffered HTTP, not streaming. See the [full parsing contract](../../pybindings/README.md#working-with-responses-and-results).

## Handle an expected 404 or an unexpected error

Use `.result()` when you want to branch on the outcome:

```python
from jsoncompat.http import Err, UnexpectedResponse
from petstore.operations.pets import GetPet404, GetPetRequest

response = await client.request(GetPetRequest(pet_id="unknown"))
match response.result():
    case Err(error=GetPet404(body=problem)):
        print("Not found:", problem.message)
    case Err(error=UnexpectedResponse() as error):
        print(error.status, error.headers.get("Retry-After"))
        print(error.text())
    case result:
        print(result.unwrap().body.name)
```

`UnexpectedResponse` preserves undeclared or malformed non-2xx responses.
Use `.body` for bytes, `.text()` for text, or `.json()` to explicitly parse JSON
without a schema. `.json()` raises `ValueError` for a non-JSON body; neither helper
discards the original bytes. `.decode_error` explains why the response could not
match the contract. Helpers decode on each call, so save their result to reuse it.

The local server has useful examples: `"unavailable"` returns a JSON 503,
`"gateway-error"` an HTML 502, `"broken-error"` an invalid 404 body, and
`"broken-success"` an invalid 200 body. Run the [full walkthrough](unexpected_responses.py):

```sh
uv run --project pybindings --extra aiohttp python examples/openapi_client/unexpected_responses.py
```

## Override one call

The adapter supplies a typed `transport=` option:

```python
from jsoncompat.http.aiohttp import AiohttpOptions

response = await client.request(
    GetPetRequest(pet_id="p_123"),
    transport=AiohttpOptions(timeout=aiohttp.ClientTimeout(total=2)),
)
```

Binding infers `AiohttpOptions` automatically. A helper that needs those controls
can accept `client: PetstoreAsyncClient[AiohttpOptions]`. Without brackets, the
alias defaults to `Never`: it accepts any adapter, while only allowing omitted
or `None` transport options. Request and response types remain precise either way.
`PetstoreClient` has the same optional type parameter.

HTTPX uses `HttpxOptions(timeout=2)`; requests uses
`RequestsOptions(timeout=(1, 2))` for connect/read timeouts. Omitting `transport`
uses the configured defaults. [Timeout details](../../pybindings/README.md#optional-http-integrations).

Extra headers and query parameters work with every adapter:

```python
from jsoncompat.http import RequestOptions

response = await client.request(
    GetPetRequest(pet_id="p_123"),
    options=RequestOptions(extra_headers={"X-Trace-ID": "demo"}),
)
```

## More examples and reference

- [Shared typed workflow](exercise.py): CRUD, pagination, optional fields, headers,
  cookies, JSON/text bodies, and expected errors across all four transports.
- [Custom fetcher contract](../../pybindings/README.md#the-fetcher-contract): the
  single function needed to connect another HTTP library.
- [Prepared request example](prepared_client.py): access encoded requests and
  decode raw responses directly when you need more control.
- [HTTP reference](../../pybindings/README.md#api-scoped-http-requests): supported
  OpenAPI features, serialization, overrides, and error behavior.

To publish a contract, ship the entire generated directory, including `py.typed`,
and depend on a jsoncompat version supporting this runtime. API preparation uses
no HTTP library and needs no session. Individual model imports remain lazy.

`cargo test --test openapi_client` checks the committed goldens, generates fresh
packages, checks static typing, and runs the examples and timeout tests against
local servers.

## Test against a public API

```sh
just python-http-live
```

This runs aiohttp, HTTPX sync/async, and requests against JSONPlaceholder: read,
filter, create, patch, verify simulated writes do not persist, and handle a 404.
There are 24 HTTPS requests, with no credentials and explicit timeouts. This
internet test is optional and is not part of `just check`.

[The spec](jsonplaceholder.json) is our OpenAPI 3.1 description of the
[documented posts API](https://jsonplaceholder.typicode.com/#guide).
[The test](live_jsonplaceholder.py) demonstrates generic helpers that preserve
the caller's transport-options type. To run an individual transport:

```sh
cargo run --locked --bin jsoncompat -- codegen \
  --openapi examples/openapi_client/jsonplaceholder.json \
  --api JsonPlaceholder --output examples/openapi_client/jsonplaceholder
uv run --project pybindings --all-extras --locked python \
  examples/openapi_client/live_jsonplaceholder.py LiveAsyncTests.test_aiohttp
```

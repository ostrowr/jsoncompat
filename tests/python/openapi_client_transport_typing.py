"""Strict positive cases for adapter inference and explicit API boundaries."""

from typing import Never, assert_type

import aiohttp
import httpx
import requests
from jsoncompat.http import (
    AsyncClient,
    AsyncFetch,
    Client,
    ClientResponse,
    EncodedRequest,
    Fetch,
    RawResponse,
    Request,
    RequestOptions,
    bind,
    bind_async,
)
from jsoncompat.http.aiohttp import AiohttpFetch, AiohttpOptions
from jsoncompat.http.httpx import AsyncHttpxFetch, HttpxFetch, HttpxOptions
from jsoncompat.http.requests import RequestsFetch, RequestsOptions
from petstore import (
    GetPetError,
    GetPetRequest,
    GetPetResult,
    GetPetSuccess,
    PetstoreAPI,
    PetstoreAsyncClient,
    PetstoreClient,
)


def portable(client: PetstoreClient) -> GetPetResult:
    assert_type(client, Client[PetstoreAPI, Never])
    return client.request(GetPetRequest(pet_id="x")).result()


async def portable_async(client: PetstoreAsyncClient) -> GetPetResult:
    assert_type(client, AsyncClient[PetstoreAPI, Never])
    return (await client.request(GetPetRequest(pet_id="x"))).result()


def fetch(
    request: EncodedRequest, /, *, transport: HttpxOptions | None = None
) -> RawResponse:
    raise NotImplementedError


async def async_fetch(
    request: EncodedRequest, /, *, transport: AiohttpOptions | None = None
) -> RawResponse:
    raise NotImplementedError


class Fake:
    def request[S, E](
        self,
        request: Request[PetstoreAPI, S, E],
        /,
        *,
        options: RequestOptions | None = None,
        transport: HttpxOptions | None = None,
    ) -> ClientResponse[S, E]:
        raise NotImplementedError


def sync_clients(session: requests.Session, httpx_session: httpx.Client) -> None:
    request = GetPetRequest(pet_id="x")
    requests_fetch: Fetch[RequestsOptions] = RequestsFetch(
        session, "https://example.com"
    )
    client = bind(PetstoreAPI, fetch=requests_fetch)
    assert_type(client, Client[PetstoreAPI, RequestsOptions])
    assert_type(
        client.request(request, transport=RequestsOptions(timeout=(1, 2))),
        ClientResponse[GetPetSuccess, GetPetError],
    )
    assert_type(portable(client), GetPetResult)
    httpx_client = bind(
        PetstoreAPI, fetch=HttpxFetch(httpx_session, "https://example.com")
    )
    assert_type(httpx_client, Client[PetstoreAPI, HttpxOptions])
    assert_type(
        httpx_client.request(request, transport=HttpxOptions(timeout=None)),
        ClientResponse[GetPetSuccess, GetPetError],
    )
    assert_type(portable(httpx_client), GetPetResult)
    assert_type(native(httpx_client), GetPetResult)
    assert_type(bind(PetstoreAPI, fetch=fetch), Client[PetstoreAPI, HttpxOptions])
    fake: PetstoreClient[HttpxOptions] = Fake()
    assert_type(portable(fake), GetPetResult)


async def async_clients(
    session: aiohttp.ClientSession, httpx_session: httpx.AsyncClient
) -> None:
    request = GetPetRequest(pet_id="x")
    aiohttp_fetch: AsyncFetch[AiohttpOptions] = AiohttpFetch(
        session, "https://example.com"
    )
    client = bind_async(PetstoreAPI, fetch=aiohttp_fetch)
    assert_type(client, AsyncClient[PetstoreAPI, AiohttpOptions])
    assert_type(
        await client.request(
            request,
            transport=AiohttpOptions(timeout=aiohttp.ClientTimeout(total=2)),
            options=RequestOptions(extra_headers={"X-Trace-ID": "one"}),
        ),
        ClientResponse[GetPetSuccess, GetPetError],
    )
    assert_type(await portable_async(client), GetPetResult)
    assert_type(await native_async(client), GetPetResult)
    httpx_client = bind_async(
        PetstoreAPI, fetch=AsyncHttpxFetch(httpx_session, "https://example.com")
    )
    assert_type(httpx_client, AsyncClient[PetstoreAPI, HttpxOptions])
    assert_type(
        await httpx_client.request(request, transport=HttpxOptions(timeout=2)),
        ClientResponse[GetPetSuccess, GetPetError],
    )
    assert_type(await portable_async(httpx_client), GetPetResult)
    assert_type(
        bind_async(PetstoreAPI, fetch=async_fetch),
        AsyncClient[PetstoreAPI, AiohttpOptions],
    )


def native(client: PetstoreClient[HttpxOptions]) -> GetPetResult:
    assert_type(client, Client[PetstoreAPI, HttpxOptions])
    return client.request(
        GetPetRequest(pet_id="x"), transport=HttpxOptions(timeout=2)
    ).result()


async def native_async(client: PetstoreAsyncClient[AiohttpOptions]) -> GetPetResult:
    assert_type(client, AsyncClient[PetstoreAPI, AiohttpOptions])
    return (
        await client.request(
            GetPetRequest(pet_id="x"),
            transport=AiohttpOptions(timeout=aiohttp.ClientTimeout(total=2)),
        )
    ).result()


async def forward[Options](
    client: PetstoreAsyncClient[Options], transport: Options
) -> GetPetResult:
    return (
        await client.request(GetPetRequest(pet_id="x"), transport=transport)
    ).result()


class AsyncFake:
    async def request[S, E](
        self,
        request: Request[PetstoreAPI, S, E],
        /,
        *,
        options: RequestOptions | None = None,
        transport: HttpxOptions | None = None,
    ) -> ClientResponse[S, E]:
        raise NotImplementedError


async def fake_async() -> GetPetResult:
    client: PetstoreAsyncClient[HttpxOptions] = AsyncFake()
    assert_type(await forward(client, HttpxOptions(timeout=2)), GetPetResult)
    return await portable_async(client)

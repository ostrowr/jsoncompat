"""Strict positive cases for adapter inference and explicit API boundaries."""

from typing import Never, assert_type

import aiohttp
import httpx
import requests
from jsoncompat.http import (
    AsyncClient,
    AsyncFetch,
    Client,
    EncodedRequest,
    Fetch,
    RawResponse,
    Request,
    RequestOptions,
    Result,
    bind,
    bind_async,
)
from jsoncompat.http.aiohttp import AiohttpFetch, AiohttpOptions
from jsoncompat.http.httpx import AsyncHttpxFetch, HttpxFetch, HttpxOptions
from jsoncompat.http.requests import RequestsFetch, RequestsOptions
from petstore import GetPetRequest, GetPetResult, PetstoreAPI


def portable(client: Client[PetstoreAPI, Never]) -> GetPetResult:
    return client.request(GetPetRequest(pet_id="x"))


async def portable_async(client: AsyncClient[PetstoreAPI, Never]) -> GetPetResult:
    return await client.request(GetPetRequest(pet_id="x"))


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
    ) -> Result[S, E]:
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
        GetPetResult,
    )
    assert_type(portable(client), GetPetResult)
    httpx_client = bind(
        PetstoreAPI, fetch=HttpxFetch(httpx_session, "https://example.com")
    )
    assert_type(httpx_client, Client[PetstoreAPI, HttpxOptions])
    assert_type(
        httpx_client.request(request, transport=HttpxOptions(timeout=None)),
        GetPetResult,
    )
    assert_type(portable(httpx_client), GetPetResult)
    assert_type(bind(PetstoreAPI, fetch=fetch), Client[PetstoreAPI, HttpxOptions])
    fake: Client[PetstoreAPI, HttpxOptions] = Fake()
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
        GetPetResult,
    )
    assert_type(await portable_async(client), GetPetResult)
    httpx_client = bind_async(
        PetstoreAPI, fetch=AsyncHttpxFetch(httpx_session, "https://example.com")
    )
    assert_type(httpx_client, AsyncClient[PetstoreAPI, HttpxOptions])
    assert_type(
        await httpx_client.request(request, transport=HttpxOptions(timeout=2)),
        GetPetResult,
    )
    assert_type(await portable_async(httpx_client), GetPetResult)
    assert_type(
        bind_async(PetstoreAPI, fetch=async_fetch),
        AsyncClient[PetstoreAPI, AiohttpOptions],
    )

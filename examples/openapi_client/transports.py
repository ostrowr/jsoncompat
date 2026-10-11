"""Small adapters around caller-owned sessions; no generated operation methods."""

from __future__ import annotations

import aiohttp
import httpx
import requests
from jsoncompat.http import EncodedRequest, RawResponse
from yarl import URL


def _url(base_url: str, request: EncodedRequest) -> str:
    # Preserve an optional API prefix and the binder's encoded path/query.
    return base_url.rstrip("/") + request.target


class AiohttpFetch:
    def __init__(self, session: aiohttp.ClientSession, base_url: str) -> None:
        self.session = session
        self.base_url = base_url

    async def __call__(self, request: EncodedRequest, /) -> RawResponse:
        # encoded=True prevents yarl from reinterpreting escaped path characters.
        async with self.session.request(
            request.method,
            URL(_url(self.base_url, request), encoded=True),
            headers=request.headers,
            data=request.body,
            skip_auto_headers={"Content-Type"},
            allow_redirects=False,
            raise_for_status=False,
        ) as response:
            return RawResponse(
                response.status,
                tuple(
                    (name.decode("ascii"), value.decode("latin-1"))
                    for name, value in response.raw_headers
                ),
                await response.read(),
            )


class HttpxFetch:
    def __init__(self, session: httpx.Client, base_url: str) -> None:
        self.session = session
        self.base_url = base_url

    def __call__(self, request: EncodedRequest, /) -> RawResponse:
        response = self.session.request(
            request.method,
            _url(self.base_url, request),
            headers=request.headers,
            content=request.body,
            follow_redirects=False,
        )
        return RawResponse(
            response.status_code,
            tuple(
                (name.decode("ascii"), value.decode("latin-1"))
                for name, value in response.headers.raw
            ),
            response.content,
        )


class AsyncHttpxFetch:
    def __init__(self, session: httpx.AsyncClient, base_url: str) -> None:
        self.session = session
        self.base_url = base_url

    async def __call__(self, request: EncodedRequest, /) -> RawResponse:
        response = await self.session.request(
            request.method,
            _url(self.base_url, request),
            headers=request.headers,
            content=request.body,
            follow_redirects=False,
        )
        return RawResponse(
            response.status_code,
            tuple(
                (name.decode("ascii"), value.decode("latin-1"))
                for name, value in response.headers.raw
            ),
            response.content,
        )


class RequestsFetch:
    def __init__(
        self, session: requests.Session, base_url: str, *, timeout: float = 10
    ) -> None:
        self.session = session
        self.base_url = base_url
        self.timeout = timeout

    def __call__(self, request: EncodedRequest, /) -> RawResponse:
        # requests accepts a mapping of request headers. Fail explicitly if an
        # adapter user supplies repeated names instead of silently losing them.
        if len({name.lower() for name, _ in request.headers}) != len(request.headers):
            raise ValueError("requests cannot send repeated request header names")
        with self.session.request(
            request.method,
            _url(self.base_url, request),
            headers=dict(request.headers),
            data=request.body,
            timeout=self.timeout,
            allow_redirects=False,
        ) as response:
            # urllib3's raw header collection retains repeated response values.
            headers = tuple(
                (name, value)
                for name in response.raw.headers
                for value in response.raw.headers.getlist(name)
            )
            return RawResponse(response.status_code, headers, response.content)

"""Optional httpx integration. Install with ``pip install 'jsoncompat[httpx]'``."""

from __future__ import annotations

from dataclasses import dataclass

import httpx

from ._metadata import EncodedRequest, RawResponse

__all__ = ["AsyncHttpxFetch", "HttpxFetch", "HttpxOptions"]


@dataclass(frozen=True, slots=True, kw_only=True)
class HttpxOptions:
    """Per-call timeout; omitting transport options inherits the fetcher's default."""

    timeout: httpx.Timeout | float | None


class HttpxFetch:
    """Fetch using a caller-owned httpx session; never create or close it.

    ``base_url`` is an absolute API root, optionally including a path prefix.
    Responses are buffered and released; redirects are returned without following.
    Native session hooks and transport exceptions propagate unchanged.
    """

    def __init__(self, session: httpx.Client, base_url: str) -> None:
        self.session = session
        self.base_url = base_url

    def __call__(
        self,
        request: EncodedRequest,
        /,
        *,
        transport: HttpxOptions | None = None,
    ) -> RawResponse:
        response = self.session.request(
            request.method,
            self.base_url.rstrip("/") + request.target,
            headers=request.headers,
            content=request.body,
            timeout=self.session.timeout if transport is None else transport.timeout,
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
    """Async equivalent of HttpxFetch using a caller-owned AsyncClient."""

    def __init__(self, session: httpx.AsyncClient, base_url: str) -> None:
        self.session = session
        self.base_url = base_url

    async def __call__(
        self,
        request: EncodedRequest,
        /,
        *,
        transport: HttpxOptions | None = None,
    ) -> RawResponse:
        response = await self.session.request(
            request.method,
            self.base_url.rstrip("/") + request.target,
            headers=request.headers,
            content=request.body,
            timeout=self.session.timeout if transport is None else transport.timeout,
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

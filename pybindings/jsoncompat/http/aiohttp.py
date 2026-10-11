"""Optional aiohttp integration. Install with ``pip install 'jsoncompat[aiohttp]'``."""

from __future__ import annotations

from dataclasses import dataclass

import aiohttp
from yarl import URL

from ._metadata import EncodedRequest, RawResponse

__all__ = ["AiohttpFetch", "AiohttpOptions"]


@dataclass(frozen=True, slots=True, kw_only=True)
class AiohttpOptions:
    """Per-call timeout; omitting transport options inherits the fetcher's default."""

    timeout: aiohttp.ClientTimeout


class AiohttpFetch:
    """Fetch using a caller-owned aiohttp session; never create or close it.

    ``base_url`` is an absolute API root, optionally including a path prefix.
    Responses are buffered and released; redirects are returned without following.
    Native session hooks and transport exceptions propagate unchanged.
    """

    def __init__(self, session: aiohttp.ClientSession, base_url: str) -> None:
        self.session = session
        self.base_url = base_url

    async def __call__(
        self,
        request: EncodedRequest,
        /,
        *,
        transport: AiohttpOptions | None = None,
    ) -> RawResponse:
        # encoded=True prevents yarl from reinterpreting escaped path characters.
        async with self.session.request(
            request.method,
            URL(self.base_url.rstrip("/") + request.target, encoded=True),
            # aiohttp tracks names case-sensitively while merging session
            # defaults. Normalize names so mixed-case duplicates stay repeated.
            headers=tuple((name.lower(), value) for name, value in request.headers),
            data=request.body,
            timeout=self.session.timeout if transport is None else transport.timeout,
            skip_auto_headers={"Content-Type"},
            allow_redirects=False,
            raise_for_status=False,
            auto_decompress=True,
        ) as response:
            return RawResponse(
                response.status,
                tuple(
                    (name.decode("ascii"), value.decode("latin-1"))
                    for name, value in response.raw_headers
                ),
                await response.read(),
            )

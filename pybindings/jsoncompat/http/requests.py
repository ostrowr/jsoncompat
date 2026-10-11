"""Optional requests integration. Install with ``pip install 'jsoncompat[requests]'``."""

from __future__ import annotations

from dataclasses import dataclass

import requests

from ._metadata import EncodedRequest, RawResponse

__all__ = ["RequestsFetch", "RequestsOptions"]


@dataclass(frozen=True, slots=True, kw_only=True)
class RequestsOptions:
    """Per-call timeout; omitting transport options inherits the fetcher's default."""

    timeout: float | tuple[float, float] | None


class RequestsFetch:
    """Fetch using a caller-owned requests session; never create or close it.

    ``base_url`` is an absolute API root, optionally including a path prefix.
    Responses are buffered and released; redirects are returned without following.
    Native session hooks and transport exceptions propagate unchanged.
    """

    def __init__(
        self,
        session: requests.Session,
        base_url: str,
        *,
        timeout: float | tuple[float, float] | None = 10,
    ) -> None:
        self.session = session
        self.base_url = base_url
        self.timeout = timeout

    def __call__(
        self,
        request: EncodedRequest,
        /,
        *,
        transport: RequestsOptions | None = None,
    ) -> RawResponse:
        # requests accepts a mapping of request headers. Fail explicitly if an
        # adapter user supplies repeated names instead of silently losing them.
        if len({name.lower() for name, _ in request.headers}) != len(request.headers):
            raise ValueError("requests cannot send repeated request header names")
        with self.session.request(
            request.method,
            self.base_url.rstrip("/") + request.target,
            headers=dict(request.headers),
            data=request.body,
            timeout=self.timeout if transport is None else transport.timeout,
            allow_redirects=False,
        ) as response:
            # urllib3's raw header collection retains repeated response values.
            headers = tuple(
                (name, value)
                for name in response.raw.headers
                for value in response.raw.headers.getlist(name)
            )
            return RawResponse(response.status_code, headers, response.content)

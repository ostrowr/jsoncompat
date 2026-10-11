"""Transport-independent HTTP contracts. Metadata never performs network I/O."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from typing import Literal, Protocol

from jsoncompat.codegen.dataclasses import DataclassModel

type HeaderPairs = tuple[tuple[str, str], ...]
type ScalarKind = Literal["string", "integer", "number", "boolean"]


@dataclass(frozen=True, slots=True)
class Path:
    name: str
    style: Literal["simple", "label", "matrix"] = "simple"
    explode: bool = False
    content_type: str | None = None


@dataclass(frozen=True, slots=True)
class Query:
    name: str
    style: Literal["form", "spaceDelimited", "pipeDelimited", "deepObject"] = "form"
    explode: bool = True
    content_type: str | None = None


@dataclass(frozen=True, slots=True)
class Header:
    name: str
    explode: bool = False
    content_type: str | None = None
    # Response headers need a declared lexical conversion before model validation.
    kind: ScalarKind = "string"
    array: bool = False


@dataclass(frozen=True, slots=True)
class Cookie:
    name: str
    explode: bool = True
    content_type: str | None = None


@dataclass(frozen=True, slots=True)
class Body:
    # None denotes explicit {content_type, value} variants for multiple media types.
    content_type: str | None = "application/json"


type Parameter = Path | Query | Header | Cookie


@dataclass(frozen=True, slots=True)
class Response:
    status: int | Literal["1XX", "2XX", "3XX", "4XX", "5XX", "default"]
    model: type[DataclassModel]
    content_type: str | None = None


@dataclass(frozen=True, slots=True)
class SecurityRequirement:
    scheme: str
    scopes: tuple[str, ...] = ()


@dataclass(frozen=True, slots=True)
class Server:
    url: str
    variables: tuple[tuple[str, str], ...] = ()


@dataclass(frozen=True, slots=True)
class Endpoint:
    method: str
    path: str
    responses: tuple[Response, ...]
    operation_id: str | None = None
    security: tuple[tuple[SecurityRequirement, ...], ...] = ()
    servers: tuple[Server, ...] = ()


def endpoint[**P, R](
    *,
    method: str,
    path: str,
    responses: tuple[Response, ...],
    operation_id: str | None = None,
    security: tuple[tuple[SecurityRequirement, ...], ...] = (),
    servers: tuple[Server, ...] = (),
) -> Callable[[Callable[P, R]], Callable[P, R]]:
    """Attach a contract while preserving the decorated method's signature."""
    return _decorator(
        Endpoint(method, path, responses, operation_id, security, servers)
    )


def _decorator[**P, R](
    metadata: Endpoint,
) -> Callable[[Callable[P, R]], Callable[P, R]]:
    def decorate(method: Callable[P, R]) -> Callable[P, R]:
        setattr(method, "__jsoncompat_endpoint__", metadata)
        return method

    return decorate


@dataclass(frozen=True, slots=True)
class EncodedRequest:
    method: str
    target: str
    headers: HeaderPairs
    body: bytes | None
    operation: Endpoint


@dataclass(frozen=True, slots=True)
class RawResponse:
    """Response bytes after HTTP transport decoding, before JSON/text parsing."""

    status: int
    headers: HeaderPairs
    body: bytes


class Fetch(Protocol):
    def __call__(self, request: EncodedRequest, /) -> RawResponse: ...


class AsyncFetch(Protocol):
    async def __call__(self, request: EncodedRequest, /) -> RawResponse: ...


class ContractError(ValueError):
    """An annotated protocol is incomplete or uses unsupported HTTP semantics."""


class ResponseError(ValueError):
    """An HTTP response does not match the declared operation contract."""

    def __init__(self, message: str, response: RawResponse) -> None:
        super().__init__(message)
        self.response = response

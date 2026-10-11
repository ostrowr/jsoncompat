"""Transport-independent HTTP contracts. Metadata never performs network I/O."""

from __future__ import annotations

import types
from dataclasses import dataclass
from typing import Generic, Literal, Protocol, TypeVar

from typing_extensions import TypeForm

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


class API:
    """Nominal identity for one generated API. Pass the class explicitly to bind."""

    __slots__ = ()

    @classmethod
    def operations(cls) -> tuple[Operation, ...]:
        raise ContractError("API must declare its registered operations")


# Keep both parameters invariant: API boundaries cannot widen, and operation
# declarations must pair a request with its exact response type.
ApiT = TypeVar("ApiT", bound=API)
ResponseT = TypeVar("ResponseT")


class Request(DataclassModel, Generic[ApiT, ResponseT]):
    """A generated operation request, scoped to one API and response type."""

    __slots__ = ()

    def __class_getitem__(cls, item: object) -> types.GenericAlias:
        # Avoid typing's global alias cache retaining generated API classes.
        return types.GenericAlias(cls, item)


class Client(Protocol[ApiT]):
    def request[R](self, request: Request[ApiT, R], /) -> R: ...


class AsyncClient(Protocol[ApiT]):
    async def request[R](self, request: Request[ApiT, R], /) -> R: ...


@dataclass(frozen=True, slots=True)
class Operation:
    """One registered request/response pair and its HTTP metadata."""

    request: type[DataclassModel]
    response: object
    endpoint: Endpoint


def operation[A: API, R](
    request: type[Request[A, R]],
    response: TypeForm[R],
    /,
    *,
    method: str,
    path: str,
    responses: tuple[Response, ...],
    operation_id: str | None = None,
    security: tuple[tuple[SecurityRequirement, ...], ...] = (),
    servers: tuple[Server, ...] = (),
) -> Operation:
    """Declare an operation while checking its request/response type pairing."""
    return Operation(
        request,
        response,
        Endpoint(method, path, responses, operation_id, security, servers),
    )


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
    """An API contract is incomplete or uses unsupported HTTP semantics."""


class ResponseError(ValueError):
    """An HTTP response does not match the declared operation contract."""

    def __init__(self, message: str, response: RawResponse) -> None:
        super().__init__(message)
        self.response = response

"""Transport-independent HTTP contracts. Metadata never performs network I/O."""

from __future__ import annotations

import types
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from typing import Generic, Literal, Protocol, TypeVar, cast

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


class Client[A: API, Options](Protocol):
    """API-scoped calls with adapter-defined transport options.

    Use Client[MyAPI, Never] at transport-independent application boundaries.
    """

    def request[R](
        self,
        request: Request[A, R],
        /,
        *,
        options: RequestOptions | None = None,
        transport: Options | None = None,
    ) -> R: ...


class AsyncClient[A: API, Options](Protocol):
    """Async counterpart of Client; transport options belong to the adapter."""

    async def request[R](
        self,
        request: Request[A, R],
        /,
        *,
        options: RequestOptions | None = None,
        transport: Options | None = None,
    ) -> R: ...


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


def _strings(value: object) -> tuple[str, ...]:
    if not isinstance(value, Sequence) or isinstance(value, (str, bytes, bytearray)):
        raise TypeError("expected a sequence of strings")
    values = tuple(cast(Sequence[object], value))
    if not all(isinstance(item, str) for item in values):
        raise TypeError("expected a sequence of strings")
    return cast(tuple[str, ...], values)


def snapshot_pairs(value: object) -> HeaderPairs:
    if isinstance(value, Mapping):
        items = tuple(cast(Mapping[object, object], value).items())
    elif isinstance(value, Sequence) and not isinstance(value, (str, bytes, bytearray)):
        items = tuple(cast(Sequence[object], value))
    else:
        raise TypeError("expected a mapping or sequence of string pairs")
    result: list[tuple[str, str]] = []
    for item in items:
        pair = _strings(item)
        if len(pair) != 2:
            raise TypeError("expected pairs of two strings")
        result.append((pair[0], pair[1]))
    return tuple(result)


@dataclass(frozen=True, slots=True, kw_only=True)
class RequestOptions:
    """Per-call wire overrides. Inputs are snapshotted; the transform runs last.

    Header names match case-insensitively. Extra headers replace existing names;
    extra query pairs append. Remove query names first to replace their values.
    Removals affect generated fields, not defaults added later by a transport.
    """

    extra_headers: Mapping[str, str] | Sequence[tuple[str, str]] = ()
    remove_headers: Sequence[str] = ()
    extra_query: Mapping[str, str] | Sequence[tuple[str, str]] = ()
    remove_query: Sequence[str] = ()
    transform: Callable[[EncodedRequest], EncodedRequest] | None = None

    def __post_init__(self) -> None:
        object.__setattr__(self, "extra_headers", snapshot_pairs(self.extra_headers))
        object.__setattr__(self, "remove_headers", _strings(self.remove_headers))
        object.__setattr__(self, "extra_query", snapshot_pairs(self.extra_query))
        object.__setattr__(self, "remove_query", _strings(self.remove_query))
        if self.transform is not None and not callable(self.transform):
            raise TypeError("transform must be callable")


@dataclass(frozen=True, slots=True)
class RawResponse:
    """Response bytes after HTTP transport decoding, before JSON/text parsing."""

    status: int
    headers: HeaderPairs
    body: bytes


class Fetch[Options](Protocol):
    """Send a buffered request. The binder always passes transport by keyword.

    None selects adapter defaults. Return every HTTP status, preserving repeated
    response headers and decoded body bytes; propagate transport exceptions.
    """

    def __call__(
        self, request: EncodedRequest, /, *, transport: Options | None = None
    ) -> RawResponse: ...


class AsyncFetch[Options](Protocol):
    """Async counterpart of Fetch; cancellation must propagate to the caller."""

    async def __call__(
        self, request: EncodedRequest, /, *, transport: Options | None = None
    ) -> RawResponse: ...


class ContractError(ValueError):
    """An API contract is incomplete or uses unsupported HTTP semantics."""


class ResponseError(ValueError):
    """An HTTP response does not match the declared operation contract."""

    def __init__(self, message: str, response: RawResponse) -> None:
        super().__init__(message)
        self.response = response

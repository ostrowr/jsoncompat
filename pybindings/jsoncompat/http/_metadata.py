"""Transport-independent HTTP contracts. Metadata never performs network I/O."""

from __future__ import annotations

import types
from collections.abc import Callable, Iterator, Mapping, Sequence
from dataclasses import dataclass, field
from threading import Lock
from typing import ClassVar, Generic, Literal, Never, Protocol, TypeVar, cast

from typing_extensions import TypeForm

from jsoncompat import JsonValue, deserialize_json_value
from jsoncompat.codegen.dataclasses import DataclassModel

type HeaderPairs = tuple[tuple[str, str], ...]
type ScalarKind = Literal["string", "integer", "number", "boolean"]


@dataclass(frozen=True, slots=True, eq=False)
class ResponseHeaders(Mapping[str, str]):
    """Read-only, case-insensitive headers. Lookup returns the first value.

    get_all() preserves every value without combining fields such as Set-Cookie.
    raw preserves the original names, order, and duplicate pairs.
    """

    raw: HeaderPairs

    def __getitem__(self, name: str) -> str:
        key = name.lower()
        for candidate, value in self.raw:
            if candidate.lower() == key:
                return value
        raise KeyError(name)

    def __iter__(self) -> Iterator[str]:
        seen: set[str] = set()
        for name, _ in self.raw:
            key = name.lower()
            if key not in seen:
                seen.add(key)
                yield name

    def __len__(self) -> int:
        return len({name.lower() for name, _ in self.raw})

    def get_all(self, name: str) -> tuple[str, ...]:
        key = name.lower()
        return tuple(value for candidate, value in self.raw if candidate.lower() == key)


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


class OperationPlan(Protocol):
    """Internal dispatch interface retained by a prepared API."""

    def encode(self, request: DataclassModel) -> EncodedRequest: ...

    def decode(self, raw: object) -> Result[DataclassModel, DataclassModel]: ...


class API:
    """Nominal identity for one generated API. Pass the class explicitly to bind."""

    __slots__ = ()

    # Owned by this exact API class, so preparation survives for its lifetime
    # without a global cache retaining generated packages or evicting plans.
    __jsoncompat_http_plans__: ClassVar[
        Mapping[type[DataclassModel], OperationPlan] | None
    ] = None

    @classmethod
    def operations(cls) -> tuple[Operation, ...]:
        raise ContractError("API must declare its registered operations")


# Keep all parameters invariant: API boundaries cannot widen, and operation
# declarations must pair a request with its exact response type.
ApiT = TypeVar("ApiT", bound=API)
ResponseT = TypeVar("ResponseT")
SuccessT = TypeVar("SuccessT")
ErrorT = TypeVar("ErrorT")


class Request(DataclassModel, Generic[ApiT, SuccessT, ErrorT]):
    """A generated operation request, scoped to one API and its success/error types."""

    __slots__ = ()

    def __class_getitem__(cls, item: object) -> types.GenericAlias:
        # Avoid typing's global alias cache retaining generated API classes.
        return types.GenericAlias(cls, item)


class Client[A: API, Options](Protocol):
    """API-scoped calls with adapter-defined transport options.

    Use Client[MyAPI, Never] at transport-independent application boundaries.
    """

    def request[S, E](
        self,
        request: Request[A, S, E],
        /,
        *,
        options: RequestOptions | None = None,
        transport: Options | None = None,
    ) -> ClientResponse[S, E]: ...


class AsyncClient[A: API, Options](Protocol):
    """Async counterpart of Client; transport options belong to the adapter."""

    async def request[S, E](
        self,
        request: Request[A, S, E],
        /,
        *,
        options: RequestOptions | None = None,
        transport: Options | None = None,
    ) -> ClientResponse[S, E]: ...


@dataclass(frozen=True, slots=True)
class Operation:
    """One registered request/response pair and its HTTP metadata."""

    request: type[DataclassModel]
    success: object
    error: object
    endpoint: Endpoint


def operation[A: API, S, E](
    request: type[Request[A, S, E]],
    success: TypeForm[S],
    error: TypeForm[E],
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
        success,
        error,
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


class HTTPStatusError(Exception):
    """A non-2xx result was unwrapped. Retains its error value and wire data."""

    def __init__(self, error: object, response: RawResponse) -> None:
        super().__init__(f"HTTP status {response.status}")
        self.error = error
        self.response = response


@dataclass(frozen=True, slots=True)
class UnexpectedResponse:
    """A non-2xx response that could not be decoded into a declared model.

    status, headers, and body expose the preserved wire data without parsing.
    text() and json() explicitly decode the bytes without validating a schema.
    decode_error retains the original contract diagnostic and its cause.
    """

    decode_error: ResponseError

    @property
    def response(self) -> RawResponse:
        return self.decode_error.response

    @property
    def status(self) -> int:
        return self.response.status

    @property
    def headers(self) -> ResponseHeaders:
        return ResponseHeaders(self.response.headers)

    @property
    def body(self) -> bytes:
        return self.response.body

    def text(
        self,
        encoding: str = "utf-8",
        *,
        errors: Literal["strict", "replace", "ignore"] = "replace",
    ) -> str:
        """Decode diagnostic text; default to UTF-8 with replacement characters."""
        return self.body.decode(encoding, errors=errors)

    def json(self) -> JsonValue:
        """Parse JSON on demand, raising ValueError for invalid JSON or UTF-8."""
        return deserialize_json_value(self.body)


@dataclass(frozen=True, slots=True)
class Ok[S]:
    """A validated 2xx response. Supports ``case Ok(value=value)``."""

    value: S

    def unwrap(self) -> S:
        """Return the success value unchanged."""
        return self.value


@dataclass(frozen=True, slots=True)
class Err[E]:
    """A non-2xx response. Inspect error or call unwrap() to raise."""

    error: E
    response: RawResponse

    def unwrap(self) -> Never:
        """Raise with the error value and original buffered response."""
        raise HTTPStatusError(self.error, self.response)


type Result[S, E] = Ok[S] | Err[E | UnexpectedResponse]


@dataclass(frozen=True, slots=True, eq=False)
class ClientResponse[S, E]:
    """Buffered response with lazy, cached model decoding; owns no connection.

    Inspect status, headers, or content without parsing. result() decodes into
    Ok/Err; unwrap() decodes and returns the success model or raises.
    Both methods share a cached result or ResponseError. Once returned, Ok.value
    and Err.error are ordinary fields; accessing them never triggers decoding.
    """

    raw: RawResponse
    _decode: Callable[[RawResponse], Result[S, E]] = field(repr=False, compare=False)
    _result: Result[S, E] | None = field(
        default=None, init=False, repr=False, compare=False
    )
    _failure: ResponseError | None = field(
        default=None, init=False, repr=False, compare=False
    )
    _lock: Lock = field(default_factory=Lock, init=False, repr=False, compare=False)

    def __post_init__(self) -> None:
        if not isinstance(cast(object, self.raw), RawResponse):
            raise TypeError("fetch must return RawResponse")

    @property
    def status(self) -> int:
        return self.raw.status

    @property
    def headers(self) -> ResponseHeaders:
        return ResponseHeaders(self.raw.headers)

    @property
    def content(self) -> bytes:
        return self.raw.body

    def result(self) -> Result[S, E]:
        """Decode once, caching both results and response contract failures."""
        with self._lock:
            if self._failure is not None:
                raise self._failure
            if self._result is None:
                try:
                    object.__setattr__(self, "_result", self._decode(self.raw))
                except ResponseError as error:
                    object.__setattr__(self, "_failure", error)
                    raise
            # A decoder returns an Ok/Err wrapper even for an empty body.
            return cast(Result[S, E], self._result)

    def unwrap(self) -> S:
        """Decode through result() and return success, or raise on HTTP/contract errors."""
        return self.result().unwrap()

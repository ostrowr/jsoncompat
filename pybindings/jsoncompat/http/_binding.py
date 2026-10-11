"""Reflect a protocol once and bind its HTTP operations to a caller-owned fetcher."""

from __future__ import annotations

import dataclasses
import inspect
import re
import types
from collections.abc import Callable, Coroutine
from dataclasses import dataclass
from functools import lru_cache, update_wrapper
from typing import (
    Annotated,
    TypeAliasType,
    Union,
    cast,
    get_args,
    get_origin,
    get_type_hints,
)
from urllib.parse import quote

from typing_extensions import TypeForm, get_protocol_members, is_protocol

from jsoncompat import JsonValue, deserialize_json_value, serialize_json_value
from jsoncompat.codegen.dataclasses import DataclassModel

from ._metadata import (
    AsyncFetch,
    Body,
    ContractError,
    Cookie,
    EncodedRequest,
    Endpoint,
    Fetch,
    Header,
    Parameter,
    Path,
    Query,
    RawResponse,
    Response,
    ResponseError,
)
from ._parameters import check_header, decode_header, encode_parameter


@dataclass(frozen=True, slots=True)
class _Field:
    name: str
    metadata: Parameter | Body


@dataclass(frozen=True, slots=True)
class _ResponsePlan:
    response: Response
    headers: tuple[_Field, ...]


@dataclass(frozen=True, slots=True)
class _Operation:
    endpoint: Endpoint
    input_model: type[DataclassModel]
    fields: tuple[_Field, ...]
    responses: tuple[_ResponsePlan, ...]
    accept: str | None
    path_template: tuple[tuple[bool, str], ...]

    def encode(self, request: DataclassModel) -> EncodedRequest:
        if type(request) is not self.input_model:
            raise TypeError(
                f"expected {self.input_model.__name__}, got {type(request).__name__}"
            )
        value = request.to_value()
        if not isinstance(value, dict):
            raise TypeError("operation inputs must be object models")
        path_values: dict[str, str] = {}
        query: list[tuple[str, str]] = []
        headers: list[tuple[str, str]] = []
        cookies: list[str] = []
        body: bytes | None = None
        for field in self.fields:
            if field.name not in value:
                continue
            item = value[field.name]
            metadata = field.metadata
            if isinstance(metadata, Body):
                content_type = metadata.content_type
                if content_type is None:
                    if (
                        not isinstance(item, dict)
                        or not isinstance(item.get("content_type"), str)
                        or "value" not in item
                    ):
                        raise ValueError(
                            "a body variant must contain content_type and value"
                        )
                    content_type = cast(str, item["content_type"])
                    item = item["value"]
                body = _encode_body(content_type, item)
                headers.append(("Content-Type", content_type))
                continue
            encoded = encode_parameter(metadata, item)
            if isinstance(metadata, Path):
                path_values[metadata.name] = cast(str, encoded)
            elif isinstance(metadata, Header):
                headers.append((metadata.name, cast(str, encoded)))
            elif isinstance(metadata, Query):
                query.extend(cast(list[tuple[str, str]], encoded))
            else:
                # RFC 6570 form expansion within each cookie parameter. Separate
                # cookie parameters use the Cookie header's semicolon delimiter.
                cookies.append(
                    "&".join(
                        f"{key}={val}"
                        for key, val in cast(list[tuple[str, str]], encoded)
                    )
                )
        target = "".join(
            path_values[value] if parameter else value
            for parameter, value in self.path_template
        )
        if any(segment in (".", "..") for segment in target.split("/")):
            raise ValueError("path parameters must not produce dot segments")
        if query:
            target += "?" + "&".join(f"{key}={val}" for key, val in query)
        if cookies:
            headers.append(("Cookie", "; ".join(cookies)))
        if self.accept is not None:
            headers.append(("Accept", self.accept))
        for name, val in headers:
            check_header(name, val, ascii_only=True)
        return EncodedRequest(
            self.endpoint.method, target, tuple(headers), body, self.endpoint
        )

    def decode(self, raw: object) -> DataclassModel:
        if not isinstance(raw, RawResponse):
            raise TypeError("fetch must return RawResponse")
        try:
            if type(raw.status) is not int or not 100 <= raw.status <= 599:
                raise ValueError("invalid HTTP response status")
            if type(raw.body) is not bytes:
                raise TypeError("response body must be bytes")
            candidates = [
                plan for plan in self.responses if plan.response.status == raw.status
            ]
            if not candidates:
                candidates = [
                    plan
                    for plan in self.responses
                    if plan.response.status == f"{raw.status // 100}XX"
                ]
            if not candidates:
                candidates = [
                    plan for plan in self.responses if plan.response.status == "default"
                ]
            if not candidates:
                raise ValueError(f"undeclared HTTP status {raw.status}")
            headers: dict[str, list[str]] = {}
            for key, header_value in raw.headers:
                check_header(key, header_value)
                headers.setdefault(key.lower(), []).append(header_value)
            content_types = headers.get("content-type", [])
            if len(content_types) > 1:
                raise ValueError("multiple Content-Type headers")
            content_type = _media_type(content_types[0]) if content_types else None
            if all(plan.response.content_type is None for plan in candidates):
                plan = candidates[0]
                if raw.body:
                    raise ValueError("response declares no body")
                body: JsonValue = None
            else:
                matching = [
                    plan
                    for plan in candidates
                    if plan.response.content_type == content_type
                ]
                if not matching:
                    raise ValueError(f"undeclared response media type {content_type!r}")
                plan = matching[0]
                body = _decode_body(cast(str, content_type), raw.body)
            typed_headers: dict[str, JsonValue] = {}
            for field in plan.headers:
                metadata = cast(Header, field.metadata)
                values = headers.get(metadata.name.lower())
                if values is not None:
                    typed_headers[field.name] = decode_header(metadata, values)
            value: dict[str, JsonValue] = {
                "status": raw.status,
                "body": body,
                "headers": typed_headers,
                "raw_headers": [[key, value] for key, value in raw.headers],
            }
            if plan.response.content_type is not None:
                value["content_type"] = content_type
            return plan.response.model.from_value(value)
        except (ValueError, TypeError, UnicodeError) as error:
            raise ResponseError(
                f"{self.endpoint.method} {self.endpoint.path}: {error}", raw
            ) from error


def _media_type(value: str) -> str:
    parts = value.split(";")
    media = parts[0].strip().lower()
    for part in parts[1:]:
        key, sep, item = part.strip().partition("=")
        if (
            sep
            and key.lower() == "charset"
            and item.strip('"').lower() not in ("utf-8", "utf8")
        ):
            raise ValueError("only UTF-8 response bodies are supported")
    return media


def _supported_media(value: str) -> bool:
    return (
        value == "text/plain"
        or value == "application/json"
        or (value.startswith("application/") and value.endswith("+json"))
    )


def _encode_body(content_type: str, value: JsonValue) -> bytes:
    if not _supported_media(content_type):
        raise ValueError(f"unsupported media type {content_type!r}")
    if content_type == "text/plain":
        if not isinstance(value, str):
            raise ValueError("text/plain requires a string")
        return value.encode("utf-8")
    return serialize_json_value(value).encode("utf-8")


def _decode_body(content_type: str, value: bytes) -> JsonValue:
    return (
        value.decode("utf-8")
        if content_type == "text/plain"
        else deserialize_json_value(value)
    )


def _fields(
    model: type[DataclassModel], *, headers: bool = False
) -> tuple[_Field, ...]:
    hints = get_type_hints(model, include_extras=True)
    result: list[_Field] = []
    for field in dataclasses.fields(model):
        if field.name == "__jsoncompat_extra__":
            raise ContractError("HTTP envelopes must reject additional properties")
        hint = hints.get(field.name)
        extras = get_args(hint)[1:] if get_origin(hint) is Annotated else ()
        metadata = [
            item
            for item in extras
            if isinstance(item, (Path, Query, Header, Cookie, Body))
        ]
        if len(metadata) != 1 or (headers and not isinstance(metadata[0], Header)):
            raise ContractError(
                f"{model.__name__}.{field.name} needs exactly one HTTP field annotation"
            )
        item = metadata[0]
        json_name = cast(str, field.metadata["jsoncompat_json_name"])
        if isinstance(item, (Path, Query, Header, Cookie)):
            if not item.name or any(c in item.name for c in "\r\n"):
                raise ContractError("invalid parameter name")
            if item.content_type is not None and (
                not _supported_media(item.content_type)
                or item.content_type == "text/plain"
            ):
                raise ContractError("parameter content must be JSON")
        if isinstance(item, Path) and field.default is not dataclasses.MISSING:
            raise ContractError("path fields must be required")
        if isinstance(item, Path) and item.style not in ("simple", "label", "matrix"):
            raise ContractError("invalid path serialization style")
        if isinstance(item, Query):
            if item.style not in (
                "form",
                "spaceDelimited",
                "pipeDelimited",
                "deepObject",
            ):
                raise ContractError("invalid query serialization style")
            if item.style == "deepObject" and not item.explode:
                raise ContractError("deepObject requires explode=True")
            if item.style in ("spaceDelimited", "pipeDelimited") and item.explode:
                raise ContractError("delimited query arrays require explode=False")
        if isinstance(item, Header):
            try:
                check_header(item.name, "")
            except ValueError as error:
                raise ContractError(str(error)) from error
            if item.kind not in ("string", "integer", "number", "boolean"):
                raise ContractError("invalid response header scalar kind")
        if (
            isinstance(item, Body)
            and item.content_type is not None
            and not _supported_media(item.content_type)
        ):
            raise ContractError(f"unsupported body media type {item.content_type}")
        result.append(_Field(json_name, item))
    return tuple(result)


def _model(value: object) -> type[DataclassModel]:
    if (
        not isinstance(value, type)
        or not issubclass(value, DataclassModel)
        or "__jsoncompat_runtime__" not in value.__dict__
    ):
        raise ContractError(
            "HTTP inputs and responses must use generated jsoncompat models"
        )
    return value


def _return_models(value: object) -> set[type[DataclassModel]]:
    if isinstance(value, TypeAliasType):
        return _return_models(value.__value__)
    if get_origin(value) in (Union, types.UnionType):
        return set[type[DataclassModel]]().union(
            *(_return_models(item) for item in get_args(value))
        )
    return {_model(value)}


def _prepare(method: Callable[..., object], asynchronous: bool) -> _Operation:
    endpoint = getattr(method, "__jsoncompat_endpoint__", None)
    if not isinstance(endpoint, Endpoint):
        raise ContractError(f"{method.__name__} needs @endpoint metadata")
    if inspect.iscoroutinefunction(method) != asynchronous:
        raise ContractError(
            "bind requires sync methods; bind_async requires async methods"
        )
    if endpoint.method not in {
        "GET",
        "PUT",
        "POST",
        "PATCH",
        "DELETE",
        "HEAD",
        "OPTIONS",
        "TRACE",
    }:
        raise ContractError(f"invalid HTTP method {endpoint.method!r}")
    if (
        not endpoint.path.startswith("/")
        or endpoint.path.startswith("//")
        or any(c in endpoint.path for c in "?#\r\n")
    ):
        raise ContractError("endpoint path must be an origin-relative path template")
    if re.search(r"[{}]", re.sub(r"\{[^{}]+\}", "", endpoint.path)):
        raise ContractError("unbalanced path template")
    signature = inspect.signature(method)
    parameters = tuple(signature.parameters.values())
    if (
        len(parameters) != 2
        or any(p.kind is not inspect.Parameter.POSITIONAL_ONLY for p in parameters)
        or any(p.default is not inspect.Parameter.empty for p in parameters)
    ):
        raise ContractError(
            "operation signatures must be (self, request: Input, /) -> Response"
        )
    hints = get_type_hints(method, include_extras=True)
    model = _model(hints.get(parameters[1].name))
    fields = _fields(model)
    paths = [
        field.metadata.name for field in fields if isinstance(field.metadata, Path)
    ]
    if len(paths) != len(set(paths)) or set(paths) != set(
        re.findall(r"\{([^{}]+)\}", endpoint.path)
    ):
        raise ContractError("path fields must match all template parameters")
    if sum(isinstance(field.metadata, Body) for field in fields) > 1:
        raise ContractError("an operation may have at most one body")
    identities = [
        (
            type(field.metadata),
            field.metadata.name.lower()
            if isinstance(field.metadata, Header)
            else field.metadata.name,
        )
        for field in fields
        if not isinstance(field.metadata, Body)
    ]
    if len(identities) != len(set(identities)):
        raise ContractError("duplicate parameter identity")
    header_names = {
        field.metadata.name.lower()
        for field in fields
        if isinstance(field.metadata, Header)
    }
    if header_names & {"content-type", "accept", "authorization"} or (
        "cookie" in header_names
        and any(isinstance(field.metadata, Cookie) for field in fields)
    ):
        raise ContractError("conflicting or reserved header parameter")
    plans: list[_ResponsePlan] = []
    selectors: set[tuple[int | str, str | None]] = set()
    for response in endpoint.responses:
        response_model = _model(response.model)
        if not (
            type(response.status) is int and 100 <= response.status <= 599
        ) and response.status not in ("1XX", "2XX", "3XX", "4XX", "5XX", "default"):
            raise ContractError("invalid response status selector")
        if response.content_type is not None and not _supported_media(
            response.content_type
        ):
            raise ContractError("unsupported response media type")
        identity = (response.status, response.content_type)
        if (
            identity in selectors
            or ((response.status, None) in selectors)
            or (
                response.content_type is None
                and any(status == response.status for status, _ in selectors)
            )
        ):
            raise ContractError("duplicate or conflicting response selector")
        selectors.add(identity)
        response_hints = get_type_hints(response_model, include_extras=True)
        required_fields = {"status", "body", "headers", "raw_headers"}
        if response.content_type is not None:
            required_fields.add("content_type")
        if {
            field.name for field in dataclasses.fields(response_model)
        } != required_fields:
            raise ContractError("invalid generated response envelope")
        header_model = _model(response_hints.get("headers"))
        plans.append(_ResponsePlan(response, _fields(header_model, headers=True)))
    if not plans or _return_models(hints.get("return")) != {
        plan.response.model for plan in plans
    }:
        raise ContractError("return annotation must match all declared response models")
    accepted = tuple(
        dict.fromkeys(
            plan.response.content_type
            for plan in plans
            if plan.response.content_type is not None
        )
    )
    path_template: list[tuple[bool, str]] = []
    for part in re.split(r"(\{[^{}]+\})", endpoint.path):
        if part.startswith("{"):
            path_template.append((True, part[1:-1]))
        else:
            if re.search(r"%(?![0-9a-fA-F]{2})", part):
                raise ContractError("invalid percent escape in endpoint path")
            path_template.append((False, quote(part, safe="/:@!$&'()*+,;=%")))
    return _Operation(
        endpoint,
        model,
        fields,
        tuple(plans),
        ", ".join(accepted) if accepted else None,
        tuple(path_template),
    )


class _SyncClient:
    __slots__ = ("_fetch",)

    def __init__(self, fetch: Fetch) -> None:
        self._fetch = fetch

    def dispatch(
        self, operation: _Operation, request: DataclassModel
    ) -> DataclassModel:
        return operation.decode(self._fetch(operation.encode(request)))


class _AsyncClient:
    __slots__ = ("_fetch",)

    def __init__(self, fetch: AsyncFetch) -> None:
        self._fetch = fetch

    async def dispatch(
        self, operation: _Operation, request: DataclassModel
    ) -> DataclassModel:
        return operation.decode(await self._fetch(operation.encode(request)))


def _sync_method(
    operation: _Operation,
) -> Callable[[_SyncClient, DataclassModel], DataclassModel]:
    def call(self: _SyncClient, request: DataclassModel, /) -> DataclassModel:
        return _SyncClient.dispatch(self, operation, request)

    return call


def _async_method(
    operation: _Operation,
) -> Callable[
    [_AsyncClient, DataclassModel], Coroutine[object, object, DataclassModel]
]:
    async def call(self: _AsyncClient, request: DataclassModel, /) -> DataclassModel:
        return await _AsyncClient.dispatch(self, operation, request)

    return call


@lru_cache(maxsize=128)
def _implementation(
    protocol: type[object], asynchronous: bool
) -> type[_SyncClient] | type[_AsyncClient]:
    if not is_protocol(protocol):
        raise ContractError("bind expects a Protocol class")
    members = get_protocol_members(protocol)
    if not members:
        raise ContractError("a service protocol must contain operations")
    methods: dict[str, object] = {"__slots__": (), "__module__": protocol.__module__}
    for name in sorted(members):
        method = getattr(protocol, name, None)
        if name.startswith("_") or not inspect.isfunction(method):
            raise ContractError(f"unsupported protocol member {name}")
        operation = _prepare(method, asynchronous)
        if asynchronous:
            implementation = update_wrapper(_async_method(operation), method)
        else:
            implementation = update_wrapper(_sync_method(operation), method)
        methods[name] = implementation
    base = _AsyncClient if asynchronous else _SyncClient
    return cast(
        type[_SyncClient] | type[_AsyncClient],
        type(f"Bound{protocol.__name__}", (base,), methods),
    )


def bind[T](protocol: TypeForm[T], /, *, fetch: Fetch) -> T:
    """Bind a synchronous generated protocol. Fetcher ownership remains with the caller."""
    if not isinstance(protocol, type):
        raise ContractError("bind expects a Protocol class")
    implementation = cast(type[_SyncClient], _implementation(protocol, False))
    return cast(T, implementation(fetch))


def bind_async[T](protocol: TypeForm[T], /, *, fetch: AsyncFetch) -> T:
    """Bind an asynchronous protocol, preserving transport exceptions and cancellation."""
    if not isinstance(protocol, type):
        raise ContractError("bind_async expects a Protocol class")
    implementation = cast(type[_AsyncClient], _implementation(protocol, True))
    return cast(T, implementation(fetch))

"""Prepare API-scoped request dispatch once for caller-owned HTTP fetchers."""

from __future__ import annotations

import dataclasses
import re
import types
from collections.abc import Callable, Mapping
from dataclasses import dataclass
from typing import (
    Annotated,
    Generic,
    Never,
    TypeAliasType,
    Union,
    cast,
    get_args,
    get_origin,
    get_type_hints,
)
from urllib.parse import quote

from jsoncompat import JsonValue, deserialize_json_value, serialize_json_value
from jsoncompat.codegen.dataclasses import DataclassModel

from ._metadata import (
    API,
    ApiT,
    AsyncClient,
    AsyncFetch,
    Body,
    Client,
    ClientResponse,
    ContractError,
    Cookie,
    EncodedRequest,
    Endpoint,
    Err,
    Fetch,
    Header,
    Ok,
    Operation,
    OperationPlan,
    Parameter,
    Path,
    Query,
    RawResponse,
    Request,
    RequestOptions,
    Response,
    ResponseError,
    ResponseT,
    Result,
    UnexpectedResponse,
)
from ._options import apply_options
from ._parameters import check_header, decode_header, encode_parameter

_DEFAULT_TO_VALUE = DataclassModel.to_value
_DEFAULT_FROM_VALUE = DataclassModel.from_value.__func__


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
        json_body = next(
            (
                field.name
                for field in self.fields
                if isinstance(field.metadata, Body)
                and field.metadata.content_type is not None
                and field.metadata.content_type != "text/plain"
            ),
            None,
        )
        encoded_body = None
        if (
            json_body is None
            or cast(object, self.input_model.to_value) is not _DEFAULT_TO_VALUE
        ):
            value = request.to_value()
        else:
            try:
                value, encoded_body = (
                    self.input_model.__jsoncompat_runtime__._serialize_json_field(
                        request, json_body
                    )
                )
            except (ValueError, TypeError):
                # Keep the value path's established validation diagnostics.
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
                body = (
                    encoded_body
                    if field.name == json_body and encoded_body is not None
                    else _encode_body(content_type, item)
                )
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

    def decode(self, raw: object) -> Result[DataclassModel, DataclassModel]:
        if not isinstance(raw, RawResponse):
            raise TypeError("fetch must return RawResponse")
        try:
            if type(raw.status) is not int or not 100 <= raw.status <= 599:
                raise ValueError("invalid HTTP response status")
            if type(raw.body) is not bytes:
                raise TypeError("response body must be bytes")
        except (ValueError, TypeError) as error:
            raise ResponseError(str(error), raw) from error
        try:
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
            else:
                matching = [
                    plan
                    for plan in candidates
                    if plan.response.content_type == content_type
                ]
                if not matching:
                    raise ValueError(f"undeclared response media type {content_type!r}")
                plan = matching[0]
            typed_headers: dict[str, JsonValue] = {}
            try:
                for field in plan.headers:
                    metadata = cast(Header, field.metadata)
                    values = headers.get(metadata.name.lower())
                    if values is not None:
                        typed_headers[field.name] = decode_header(metadata, values)
            except (ValueError, TypeError, UnicodeError):
                if plan.response.content_type is not None:
                    # The original decoder checks body syntax before headers.
                    _decode_body(cast(str, content_type), raw.body)
                raise
            value: dict[str, JsonValue] = {
                "status": raw.status,
                "headers": typed_headers,
                "raw_headers": [[key, value] for key, value in raw.headers],
            }
            if plan.response.content_type is not None:
                value["content_type"] = content_type
            decoded = _decode_model(plan.response, value, raw.body)
            return Ok(decoded) if 200 <= raw.status < 300 else Err(decoded, raw)
        except (ValueError, TypeError, UnicodeError) as error:
            failure = ResponseError(
                f"{self.endpoint.method} {self.endpoint.path}: {error}", raw
            )
            if not 200 <= raw.status < 300:
                failure.__cause__ = error
                return Err(UnexpectedResponse(failure), raw)
            raise failure from error


def _decode_model(
    response: Response, fields: dict[str, JsonValue], body: bytes
) -> DataclassModel:
    content_type = response.content_type
    if (
        content_type is not None
        and content_type != "text/plain"
        and getattr(response.model.from_value, "__func__", None) is _DEFAULT_FROM_VALUE
    ):
        try:
            decoded = response.model.__jsoncompat_runtime__._deserialize_json_field(
                fields, "body", body
            )
            if decoded is not None:
                return cast(DataclassModel, decoded)
        except (ValueError, TypeError):
            # Failed optimized decoding must have the same diagnostics and
            # UnexpectedResponse behavior as the ordinary value path.
            pass
    fields["body"] = (
        None if content_type is None else _decode_body(content_type, body)
    )
    return response.model.from_value(fields)


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
    if value is Never:
        return set()
    if isinstance(value, TypeAliasType):
        return _return_models(value.__value__)
    if get_origin(value) in (Union, types.UnionType):
        return set[type[DataclassModel]]().union(
            *(_return_models(item) for item in get_args(value))
        )
    return {_model(value)}


def _request_model(value: object, api: type[API]) -> type[DataclassModel]:
    model = _model(value)
    bases = [
        base for base in types.get_original_bases(model) if get_origin(base) is Request
    ]
    if (
        len(bases) != 1
        or len(get_args(bases[0])) != 3
        or get_args(bases[0])[0] is not api
    ):
        raise ContractError(
            f"{model.__name__} must be a Request scoped to {api.__name__}"
        )
    return model


def _prepare(spec: Operation, api: type[API]) -> _Operation:
    model = _request_model(spec.request, api)
    endpoint = cast(object, spec.endpoint)
    if not isinstance(endpoint, Endpoint):
        raise ContractError("operation needs Endpoint metadata")
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
    # Resolve status precedence before partitioning models. A default model may
    # appear in both sets, or be unreachable when ranges cover all statuses.
    by_status: dict[int | str, set[type[DataclassModel]]] = {}
    for plan in plans:
        by_status.setdefault(plan.response.status, set()).add(plan.response.model)
    success_models: set[type[DataclassModel]] = set()
    error_models: set[type[DataclassModel]] = set()
    for status in range(100, 600):
        models = by_status.get(
            status, by_status.get(f"{status // 100}XX", by_status.get("default", set()))
        )
        (success_models if 200 <= status < 300 else error_models).update(models)
    if (
        not plans
        or _return_models(spec.success) != success_models
        or _return_models(spec.error) != error_models
    ):
        raise ContractError(
            "success and error types must match reachable response models"
        )
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


def install_generated_api(
    api: type[API], version: int, operations: tuple[_Operation, ...]
) -> None:
    """Bind codegen-validated dispatch plans to this process's model classes.

    The generator checks HTTP contracts and emits field mappings, header plans,
    Accept values and escaped path segments. Import only checks the format and
    class identities; it never resolves annotations or repartitions statuses.
    Publish atomically so an invalid registry cannot leave a partly bound API.
    """
    if version != 1:
        raise ContractError("Unsupported generated HTTP plan format; regenerate client")
    if (
        "__jsoncompat_http_plans__" in api.__dict__
        and api.__jsoncompat_http_plans__ is not None
    ):
        return
    if not operations:
        raise ContractError("API must register a nonempty tuple of operations")
    plans: dict[type[DataclassModel], OperationPlan] = {}
    for plan in operations:
        model = _request_model(plan.input_model, api)
        if model in plans:
            raise ContractError(f"duplicate operation for {model.__name__}")
        plans[model] = plan
    api.__jsoncompat_http_plans__ = types.MappingProxyType(plans)


def initialize_api(api: type[API]) -> Mapping[type[DataclassModel], OperationPlan]:
    """Reuse generated plans, or validate metadata for a manually declared API."""
    # Do not inherit another API's plans: request registration is nominal.
    if (
        "__jsoncompat_http_plans__" in api.__dict__
        and api.__jsoncompat_http_plans__ is not None
    ):
        return api.__jsoncompat_http_plans__
    operations = cast(object, api.operations())
    # Loading the private identity can import the public API module, whose
    # final statement installs its generated plans. Reuse that completed plan.
    if (
        "__jsoncompat_http_plans__" in api.__dict__
        and api.__jsoncompat_http_plans__ is not None
    ):
        return api.__jsoncompat_http_plans__
    if not isinstance(operations, tuple) or not operations:
        raise ContractError("API must register a nonempty tuple of operations")
    plans: dict[type[DataclassModel], OperationPlan] = {}
    for spec in cast(tuple[object, ...], operations):
        if not isinstance(spec, Operation):
            raise ContractError("API operations must contain Operation declarations")
        plan = _prepare(spec, api)
        if plan.input_model in plans:
            raise ContractError(f"duplicate operation for {plan.input_model.__name__}")
        plans[plan.input_model] = plan
    prepared = types.MappingProxyType(plans)
    # Publish only after the entire registry validates successfully.
    api.__jsoncompat_http_plans__ = prepared
    return prepared


def _select(
    api: type[API],
    plans: Mapping[type[DataclassModel], OperationPlan],
    request: DataclassModel,
) -> OperationPlan:
    plan = plans.get(type(request))
    if plan is None:
        raise TypeError(
            f"{type(request).__name__} is not a registered request for {api.__name__}"
        )
    return plan


class _SyncClient[A: API, Options]:
    __slots__ = ("_api", "_fetch", "_plans")

    def __init__(self, api: type[A], fetch: Fetch[Options]) -> None:
        self._api = api
        self._fetch = fetch
        self._plans = initialize_api(api)

    def request[S, E](
        self,
        request: Request[A, S, E],
        /,
        *,
        options: RequestOptions | None = None,
        transport: Options | None = None,
    ) -> ClientResponse[S, E]:
        call = _prepare_request(self._api, self._plans, request, options)
        return ClientResponse(
            self._fetch(call.request, transport=transport), call.decode
        )


class _AsyncClient[A: API, Options]:
    __slots__ = ("_api", "_fetch", "_plans")

    def __init__(self, api: type[A], fetch: AsyncFetch[Options]) -> None:
        self._api = api
        self._fetch = fetch
        self._plans = initialize_api(api)

    async def request[S, E](
        self,
        request: Request[A, S, E],
        /,
        *,
        options: RequestOptions | None = None,
        transport: Options | None = None,
    ) -> ClientResponse[S, E]:
        call = _prepare_request(self._api, self._plans, request, options)
        return ClientResponse(
            await self._fetch(call.request, transport=transport), call.decode
        )


def _require_api(api: object) -> None:
    if not isinstance(api, type) or api is API or not issubclass(api, API):
        raise ContractError("expected an explicit generated API class")


@dataclass(frozen=True, slots=True)
class PreparedRequest(Generic[ApiT, ResponseT]):
    """A wire request with its API identity and validated response decoder.

    Create with prepare(). Preparation performs no I/O; decode() can be called
    after any caller-owned transport returns a buffered RawResponse.
    """

    api: type[ApiT]
    request: EncodedRequest
    _decode: Callable[[RawResponse], ResponseT] = dataclasses.field(repr=False)

    def decode(self, response: RawResponse, /) -> ResponseT:
        return self._decode(response)


def _prepare_request[A: API, S, E](
    api: type[A],
    plans: Mapping[type[DataclassModel], OperationPlan],
    request: Request[A, S, E],
    options: RequestOptions | None,
) -> PreparedRequest[A, Result[S, E]]:
    operation = _select(api, plans, request)
    encoded = apply_options(operation.encode(request), options)
    # The registered operation validates both sides of the result type.
    return PreparedRequest(
        api, encoded, cast(Callable[[RawResponse], Result[S, E]], operation.decode)
    )


def prepare[A: API, S, E](
    api: type[A], request: Request[A, S, E], /, *, options: RequestOptions | None = None
) -> PreparedRequest[A, Result[S, E]]:
    """Encode a registered request and retain its typed decoder, without I/O."""
    _require_api(api)
    return _prepare_request(api, initialize_api(api), request, options)


def bind[A: API, Options](
    api: type[A], /, *, fetch: Fetch[Options]
) -> Client[A, Options]:
    """Bind an explicit API, inferring transport options from the fetcher.

    Generated APIs prepare their dispatch plans on import; binding reuses them.
    Only registered requests reach fetch(request, transport=transport).
    The adapter owns the option type, defaults, and runtime validation.
    """
    _require_api(api)
    return _SyncClient(api, fetch)


def bind_async[A: API, Options](
    api: type[A], /, *, fetch: AsyncFetch[Options]
) -> AsyncClient[A, Options]:
    """Attach an async fetcher to an API's prepared plans, preserving cancellation."""
    _require_api(api)
    return _AsyncClient(api, fetch)

"""OpenAPI parameter serialization for flat scalar, array, and object values."""

from __future__ import annotations

import json
import re
from decimal import Decimal
from urllib.parse import quote

from jsoncompat import JsonValue, deserialize_json_value, serialize_json_value

from ._metadata import Header, Parameter, Path, Query

_TOKEN = re.compile(r"[!#$%&'*+.^_`|~0-9A-Za-z-]+\Z")


def check_header(name: str, value: str, *, ascii_only: bool = False) -> None:
    if not _TOKEN.fullmatch(name) or any(
        ord(c) < 32 and c != "\t" or ord(c) == 127 for c in value
    ):
        raise ValueError(f"invalid HTTP header {name!r}")
    try:
        value.encode("ascii" if ascii_only else "latin-1")
    except UnicodeEncodeError as error:
        encoding = "ASCII" if ascii_only else "Latin-1"
        raise ValueError(f"header {name!r} must be {encoding}") from error


def scalar(value: JsonValue) -> str:
    if isinstance(value, str):
        return value
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float, Decimal)):
        return serialize_json_value(value)
    raise ValueError(
        "non-JSON parameters require non-null scalars or flat arrays/objects; use content: application/json for structured values"
    )


def pieces(value: JsonValue, encode: bool) -> str | list[str] | dict[str, str]:
    def text(item: JsonValue) -> str:
        result = scalar(item)
        return quote(result, safe="") if encode else result

    if isinstance(value, dict):
        return {text(key): text(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [text(item) for item in value]
    return text(value)


def simple(
    value: str | list[str] | dict[str, str], explode: bool, separator: str = ","
) -> str:
    if isinstance(value, str):
        return value
    if isinstance(value, dict):
        return (
            separator.join(f"{key}={item}" for key, item in value.items())
            if explode
            else separator.join(part for pair in value.items() for part in pair)
        )
    return separator.join(value)


def encode_parameter(
    parameter: Parameter, value: JsonValue
) -> str | list[tuple[str, str]]:
    if parameter.content_type is not None:
        value = serialize_json_value(value)
        if isinstance(parameter, Header):
            # Escape Unicode inside JSON strings without re-parsing numbers or
            # losing decimal precision. JSON escapes are portable in HTTP headers.
            value = "".join(
                json.dumps(char)[1:-1] if ord(char) > 127 else char for char in value
            )
    converted = pieces(value, not isinstance(parameter, Header))
    name = quote(parameter.name, safe="")
    if isinstance(parameter, Path):
        if parameter.style == "label":
            return "." + simple(
                converted, parameter.explode, "." if parameter.explode else ","
            )
        if parameter.style == "matrix":
            if isinstance(converted, dict) and parameter.explode:
                return "".join(f";{key}={item}" for key, item in converted.items())
            if isinstance(converted, list) and parameter.explode:
                return "".join(f";{name}={item}" for item in converted)
            result = simple(converted, False)
            return f";{name}={result}" if result else f";{name}"
        return simple(converted, parameter.explode)
    if isinstance(parameter, Header):
        result = simple(converted, parameter.explode)
        check_header(parameter.name, result, ascii_only=True)
        return result
    if isinstance(parameter, Query):
        if parameter.style == "deepObject":
            if not isinstance(converted, dict):
                raise ValueError("deepObject requires a flat object")
            return [(f"{name}%5B{key}%5D", item) for key, item in converted.items()]
        if parameter.style in ("spaceDelimited", "pipeDelimited"):
            if not isinstance(converted, list) or parameter.explode:
                raise ValueError(
                    f"{parameter.style} requires an array and explode=False"
                )
            separator = "%20" if parameter.style == "spaceDelimited" else "|"
            return [(name, separator.join(converted))]
    if isinstance(converted, dict) and parameter.explode:
        return list(converted.items())
    if isinstance(converted, list) and parameter.explode:
        return [(name, item) for item in converted]
    return [(name, simple(converted, False))]


def decode_header(parameter: Header, values: list[str]) -> JsonValue:
    if parameter.content_type is not None:
        if len(values) != 1:
            raise ValueError(f"expected one JSON header {parameter.name}")
        return deserialize_json_value(values[0])

    def decode(value: str) -> JsonValue:
        if parameter.kind == "string":
            return value
        if parameter.kind == "boolean":
            if value not in ("true", "false"):
                raise ValueError(f"invalid boolean header {parameter.name}")
            return value == "true"
        if parameter.kind == "integer":
            if not re.fullmatch(r"-?(?:0|[1-9][0-9]*)", value):
                raise ValueError(f"invalid integer header {parameter.name}")
            return int(value)
        result = deserialize_json_value(value)
        if isinstance(result, bool) or not isinstance(result, (int, float, Decimal)):
            raise ValueError(f"invalid number header {parameter.name}")
        return result

    if parameter.array:
        if parameter.name.lower() == "set-cookie":
            return [decode(value) for value in values]
        return [decode(part.strip()) for value in values for part in value.split(",")]
    if len(values) != 1:
        raise ValueError(f"expected one scalar header {parameter.name}")
    return decode(values[0])

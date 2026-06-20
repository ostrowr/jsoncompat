from __future__ import annotations

import functools
import importlib
import math
from enum import StrEnum
from typing import Any, Literal, NoReturn, cast, overload

from jsoncompat import JsonValue, deserialize_json_value, serialize_json_value


__all__ = [
    "SerializationFormat",
    "deserialize_value",
    "serialize_value",
]


class SerializationFormat(StrEnum):
    JSON = "json"
    YAML = "yaml"
    MSGPACK = "msgpack"


@overload
def serialize_value(
    value: JsonValue,
    *,
    format: Literal[SerializationFormat.JSON] = SerializationFormat.JSON,
) -> str: ...


@overload
def serialize_value(
    value: JsonValue,
    *,
    format: Literal[SerializationFormat.YAML],
) -> str: ...


@overload
def serialize_value(
    value: JsonValue,
    *,
    format: Literal[SerializationFormat.MSGPACK],
) -> bytes: ...


@overload
def serialize_value(
    value: JsonValue,
    *,
    format: SerializationFormat,
) -> str | bytes: ...


def serialize_value(
    value: JsonValue,
    *,
    format: SerializationFormat = SerializationFormat.JSON,
) -> str | bytes:
    selected_format = SerializationFormat(format)

    if selected_format is SerializationFormat.JSON:
        return serialize_json_value(value)

    normalized = _normalize_json_value(value)
    if selected_format is SerializationFormat.YAML:
        yaml = _optional_module("yaml", "yaml")
        encoded = yaml.safe_dump(
            normalized,
            allow_unicode=True,
            sort_keys=True,
        )
        if not isinstance(encoded, str):
            raise TypeError("YAML encoder returned a non-string value")
        return encoded

    msgpack = _optional_module("msgpack", "msgpack")
    encoded = msgpack.packb(normalized, use_bin_type=True, strict_types=True)
    if not isinstance(encoded, bytes):
        raise TypeError("MessagePack encoder returned a non-bytes value")
    return encoded


def deserialize_value(
    payload: str | bytes,
    *,
    format: SerializationFormat = SerializationFormat.JSON,
) -> JsonValue:
    selected_format = SerializationFormat(format)

    if selected_format is SerializationFormat.JSON:
        return deserialize_json_value(payload)
    elif selected_format is SerializationFormat.YAML:
        yaml = _optional_module("yaml", "yaml")
        decoded = yaml.load(payload, Loader=_yaml_loader_type())
    else:
        if not isinstance(payload, bytes):
            raise TypeError("MessagePack payloads must be bytes")
        msgpack = _optional_module("msgpack", "msgpack")
        decoded = msgpack.unpackb(
            payload,
            raw=False,
            strict_map_key=True,
            object_pairs_hook=_unique_mapping,
            ext_hook=_reject_msgpack_extension,
        )

    return _normalize_json_value(decoded)


def _optional_module(module_name: str, extra_name: str) -> Any:
    try:
        return importlib.import_module(module_name)
    except ModuleNotFoundError as error:
        if error.name != module_name:
            raise
        raise ModuleNotFoundError(
            f"{module_name} support requires the optional jsoncompat[{extra_name}] "
            "dependency"
        ) from None


def _unique_mapping(pairs: list[tuple[Any, Any]]) -> dict[Any, Any]:
    output: dict[Any, Any] = {}
    for key, value in pairs:
        try:
            duplicate = key in output
        except TypeError:
            raise ValueError("serialized mapping keys must be hashable") from None
        if duplicate:
            raise ValueError(f"duplicate serialized mapping key {key!r}")
        output[key] = value
    return output


def _yaml_unique_mapping(
    loader: Any,
    node: Any,
    deep: bool = False,
) -> dict[Any, Any]:
    loader.flatten_mapping(node)
    return _unique_mapping(loader.construct_pairs(node, deep=deep))


@functools.lru_cache(maxsize=1)
def _yaml_loader_type() -> Any:
    yaml = _optional_module("yaml", "yaml")
    loader_type = type(
        "JsoncompatSafeLoader",
        (yaml.SafeLoader,),
        {},
    )
    loader_type.add_constructor(
        yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG,
        _yaml_unique_mapping,
    )
    return loader_type


def _reject_msgpack_extension(code: int, data: bytes) -> NoReturn:
    _ = data
    raise ValueError(f"MessagePack extension type {code} is not a JSON value")


def _normalize_json_value(
    value: Any,
    *,
    path: str = "$",
    active_containers: set[int] | None = None,
) -> JsonValue:
    if value is None or isinstance(value, (bool, str)):
        return value
    if isinstance(value, int):
        return value
    if isinstance(value, float):
        if not math.isfinite(value):
            raise ValueError(f"{path} contains a non-finite number")
        return value

    if active_containers is None:
        active_containers = set()

    if isinstance(value, (list, tuple)):
        sequence = cast(list[Any] | tuple[Any, ...], value)
        container_id = id(sequence)
        if container_id in active_containers:
            raise ValueError(f"{path} contains a cyclic sequence")
        active_containers.add(container_id)
        try:
            return [
                _normalize_json_value(
                    item,
                    path=f"{path}[{index}]",
                    active_containers=active_containers,
                )
                for index, item in enumerate(sequence)
            ]
        finally:
            active_containers.remove(container_id)

    if isinstance(value, dict):
        mapping = cast(dict[Any, Any], value)
        container_id = id(mapping)
        if container_id in active_containers:
            raise ValueError(f"{path} contains a cyclic mapping")
        active_containers.add(container_id)
        try:
            output: dict[str, JsonValue] = {}
            for key, item in mapping.items():
                if not isinstance(key, str):
                    raise TypeError(
                        f"{path} contains non-string mapping key {key!r}"
                    )
                output[key] = _normalize_json_value(
                    item,
                    path=f"{path}.{key}",
                    active_containers=active_containers,
                )
            return output
        finally:
            active_containers.remove(container_id)

    raise TypeError(f"{path} contains non-JSON value {type(value).__name__}")

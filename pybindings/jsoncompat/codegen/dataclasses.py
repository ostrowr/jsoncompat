from __future__ import annotations

import dataclasses
import functools
import json
import types
from typing import (
    Any,
    ClassVar,
    Literal,
    NoReturn,
    Union,
    cast,
    get_args,
    get_origin,
    get_type_hints,
)

from jsoncompat import is_valid


__all__ = [
    "DataclassAdditionalModel",
    "DataclassModel",
    "DataclassRootModel",
    "JSONCOMPAT_EXTRA_FIELD",
    "JSONCOMPAT_MISSING",
    "JsoncompatMissingType",
    "ReaderDataclassModel",
    "ReaderDataclassRootModel",
    "Omittable",
    "WriterDataclassModel",
    "extra_field",
    "field",
    "root_field",
]


JSONCOMPAT_EXTRA_FIELD = "__jsoncompat_extra__"
JSONCOMPAT_SCHEMA_FIELD = "__jsoncompat_schema__"
JSONCOMPAT_JSON_NAME_METADATA = "jsoncompat_json_name"
JSONCOMPAT_MISSING_METADATA = "jsoncompat_omittable"
_JSONCOMPAT_MISSING_TYPE_HINT = object()


class JsoncompatMissingType:
    __slots__ = ()

    def __repr__(self) -> str:
        return "JSONCOMPAT_MISSING"


JSONCOMPAT_MISSING = JsoncompatMissingType()

type Omittable[T] = T | JsoncompatMissingType


@dataclasses.dataclass(frozen=True, slots=True)
class _JsoncompatFieldSpec:
    py_name: str
    json_name: str
    annotation: Any
    omittable: bool


@dataclasses.dataclass(frozen=True, slots=True)
class _JsoncompatObjectSpec:
    fields: tuple[_JsoncompatFieldSpec, ...]
    known_json_names: frozenset[str]
    extra_annotation: Any | None


def field(
    json_name: str,
    *,
    omittable: bool = False,
) -> Any:
    metadata = {
        JSONCOMPAT_JSON_NAME_METADATA: json_name,
        JSONCOMPAT_MISSING_METADATA: omittable,
    }
    if omittable:
        return dataclasses.field(default=JSONCOMPAT_MISSING, metadata=metadata)
    return dataclasses.field(metadata=metadata)


def extra_field() -> Any:
    return dataclasses.field(default_factory=_jsoncompat_empty_extra, repr=False)


def root_field() -> Any:
    return dataclasses.field()


class DataclassModel:
    __slots__ = ()

    __jsoncompat_schema__: ClassVar[str]

    def __post_init__(self) -> None:
        schema_json = _jsoncompat_schema_for(type(self))
        if schema_json is None:
            return
        value = self.jsoncompat_to_json_unchecked()
        if not is_valid(schema_json, json.dumps(value, separators=(",", ":"), sort_keys=True)):
            raise ValueError(
                f"{type(self).__name__} instance does not satisfy its JSON Schema"
            )

    @classmethod
    def from_json[JSONCOMPAT_MODEL_T: DataclassModel](
        cls: type[JSONCOMPAT_MODEL_T],
        value: Any,
    ) -> JSONCOMPAT_MODEL_T:
        return cls.jsoncompat_from_json_checked(value)

    @classmethod
    def from_json_string[JSONCOMPAT_MODEL_T: DataclassModel](
        cls: type[JSONCOMPAT_MODEL_T],
        value_json: str,
    ) -> JSONCOMPAT_MODEL_T:
        return cls.from_json(json.loads(value_json))

    def to_json(self) -> Any:
        return self.jsoncompat_to_json_checked()

    def to_json_string(self) -> str:
        return json.dumps(self.to_json(), separators=(",", ":"), sort_keys=True)

    @classmethod
    def jsoncompat_from_json_checked[JSONCOMPAT_MODEL_T: DataclassModel](
        cls: type[JSONCOMPAT_MODEL_T],
        value: Any,
    ) -> JSONCOMPAT_MODEL_T:
        schema_json = _jsoncompat_schema_for(cls)
        if schema_json is None:
            raise TypeError(f"{cls.__name__} is missing __jsoncompat_schema__")
        instance_json = json.dumps(value, separators=(",", ":"), sort_keys=True)
        if not is_valid(schema_json, instance_json):
            raise ValueError(f"value does not satisfy {cls.__name__} schema")
        return cls.jsoncompat_from_validated(value)

    @classmethod
    def jsoncompat_from_validated[JSONCOMPAT_MODEL_T: DataclassModel](
        cls: type[JSONCOMPAT_MODEL_T],
        value: Any,
    ) -> JSONCOMPAT_MODEL_T:
        if not isinstance(value, dict):
            raise TypeError(f"{cls.__name__} expects a JSON object")

        value_object = cast(dict[str, Any], value)
        object_spec = _jsoncompat_object_spec_for(cls)
        kwargs: dict[str, Any] = {}
        for field_spec in object_spec.fields:
            if field_spec.json_name not in value_object:
                if field_spec.omittable:
                    kwargs[field_spec.py_name] = JSONCOMPAT_MISSING
                continue
            kwargs[field_spec.py_name] = _jsoncompat_construct_value(
                field_spec.annotation,
                value_object[field_spec.json_name],
            )

        if object_spec.extra_annotation is not None:
            kwargs[JSONCOMPAT_EXTRA_FIELD] = _jsoncompat_construct_extra(
                object_spec.extra_annotation,
                {
                    key: item
                    for key, item in value_object.items()
                    if key not in object_spec.known_json_names
                },
            )

        return _jsoncompat_new_unchecked(cls, kwargs)

    def jsoncompat_to_json_checked(self) -> Any:
        value = self.jsoncompat_to_json_unchecked()
        schema_json = _jsoncompat_schema_for(type(self))
        if schema_json is None:
            raise TypeError(f"{type(self).__name__} is missing __jsoncompat_schema__")
        instance_json = json.dumps(value, separators=(",", ":"), sort_keys=True)
        if not is_valid(schema_json, instance_json):
            raise ValueError(
                f"{type(self).__name__} instance does not satisfy its JSON Schema"
            )
        return value

    def jsoncompat_to_json_unchecked(self) -> Any:
        output: dict[str, Any] = {}
        for field in _jsoncompat_dataclass_fields(self):
            field_value = getattr(self, field.name)
            if field.name == JSONCOMPAT_EXTRA_FIELD:
                output.update(_jsoncompat_serialize_value(field_value))
                continue
            if field_value is JSONCOMPAT_MISSING:
                continue
            json_name = field.metadata.get(JSONCOMPAT_JSON_NAME_METADATA, field.name)
            output[json_name] = _jsoncompat_serialize_value(field_value)
        return output


class DataclassAdditionalModel[JSONCOMPAT_ADDITIONAL_T](DataclassModel):
    __slots__ = ()

    __jsoncompat_extra__: dict[str, JSONCOMPAT_ADDITIONAL_T]

    def get_additional_property(
        self,
        json_name: str,
    ) -> JSONCOMPAT_ADDITIONAL_T | JsoncompatMissingType:
        return self.__jsoncompat_extra__.get(json_name, JSONCOMPAT_MISSING)


class DataclassRootModel(DataclassModel):
    __slots__ = ()

    root: Any

    @classmethod
    def jsoncompat_from_validated[JSONCOMPAT_ROOT_MODEL_T: DataclassRootModel](
        cls: type[JSONCOMPAT_ROOT_MODEL_T],
        value: Any,
    ) -> JSONCOMPAT_ROOT_MODEL_T:
        return _jsoncompat_new_unchecked(
            cls,
            {
                "root": _jsoncompat_construct_value(
                    _jsoncompat_root_annotation_for(cls),
                    value,
                )
            },
        )

    def jsoncompat_to_json_unchecked(self) -> Any:
        return _jsoncompat_serialize_value(self.root)


class ReaderDataclassModel(DataclassModel):
    __slots__ = ()

    def to_json(self) -> NoReturn:
        raise TypeError("Reader dataclasses do not support serialization")

    def to_json_string(self) -> NoReturn:
        raise TypeError("Reader dataclasses do not support serialization")


class ReaderDataclassRootModel(DataclassRootModel):
    __slots__ = ()

    def to_json(self) -> NoReturn:
        raise TypeError("Reader dataclasses do not support serialization")

    def to_json_string(self) -> NoReturn:
        raise TypeError("Reader dataclasses do not support serialization")


class WriterDataclassModel(DataclassModel):
    __slots__ = ()

    @classmethod
    def from_json(cls, value: Any) -> NoReturn:
        _ = value
        raise TypeError("Writer dataclasses do not support deserialization")

    @classmethod
    def from_json_string(cls, value_json: str) -> NoReturn:
        _ = value_json
        raise TypeError("Writer dataclasses do not support deserialization")


def _jsoncompat_schema_for(model_type: type[DataclassModel]) -> str | None:
    schema = getattr(model_type, JSONCOMPAT_SCHEMA_FIELD, None)
    if schema is None:
        return None
    if not isinstance(schema, str):
        raise TypeError(
            f"{model_type.__name__}.{JSONCOMPAT_SCHEMA_FIELD} must be a JSON string"
        )
    return schema


@functools.lru_cache(maxsize=None)
def _jsoncompat_type_hints_for(model_type: type[Any]) -> dict[str, Any]:
    return get_type_hints(model_type)


@functools.lru_cache(maxsize=None)
def _jsoncompat_object_spec_for(
    model_type: type[DataclassModel],
) -> _JsoncompatObjectSpec:
    fields: list[_JsoncompatFieldSpec] = []
    known_json_names: set[str] = set()
    extra_annotation: Any | None = None
    type_hints = _jsoncompat_type_hints_for(model_type)

    for field in _jsoncompat_dataclass_fields(model_type):
        if field.name == JSONCOMPAT_EXTRA_FIELD:
            extra_annotation = _jsoncompat_type_hint_for(model_type, type_hints, field.name)
            continue
        json_name = field.metadata.get(JSONCOMPAT_JSON_NAME_METADATA, field.name)
        omittable = field.metadata.get(JSONCOMPAT_MISSING_METADATA, False)
        annotation = _jsoncompat_type_hint_for(model_type, type_hints, field.name)
        if omittable:
            annotation = _jsoncompat_runtime_annotation(annotation)
        fields.append(
            _JsoncompatFieldSpec(
                py_name=field.name,
                json_name=json_name,
                annotation=annotation,
                omittable=omittable,
            )
        )
        known_json_names.add(json_name)

    return _JsoncompatObjectSpec(
        fields=tuple(fields),
        known_json_names=frozenset(known_json_names),
        extra_annotation=extra_annotation,
    )


def _jsoncompat_type_hint_for(
    model_type: type[Any],
    type_hints: dict[str, Any],
    field_name: str,
) -> Any:
    annotation = type_hints.get(field_name, _JSONCOMPAT_MISSING_TYPE_HINT)
    if annotation is _JSONCOMPAT_MISSING_TYPE_HINT:
        raise TypeError(f"{model_type.__name__}.{field_name} is missing a type annotation")
    return annotation


def _jsoncompat_runtime_annotation(annotation: Any) -> Any:
    if get_origin(annotation) is Omittable:
        args = get_args(annotation)
        if len(args) != 1:
            raise TypeError("Omittable annotations must have exactly one argument")
        return args[0] | JsoncompatMissingType
    return annotation


@functools.lru_cache(maxsize=None)
def _jsoncompat_root_annotation_for(model_type: type[DataclassRootModel]) -> Any:
    type_hints = _jsoncompat_type_hints_for(model_type)
    return _jsoncompat_runtime_annotation(
        _jsoncompat_type_hint_for(model_type, type_hints, "root")
    )


def _jsoncompat_construct_extra(annotation: Any, value: dict[str, Any]) -> dict[str, Any]:
    origin = get_origin(annotation)
    if origin is dict:
        args = get_args(annotation)
        value_annotation = args[1] if len(args) == 2 else Any
    else:
        value_annotation = Any
    return {
        key: _jsoncompat_construct_value(value_annotation, item)
        for key, item in value.items()
    }


def _jsoncompat_construct_value(annotation: Any, value: Any) -> Any:
    if annotation is Any:
        return value
    if annotation is JsoncompatMissingType:
        if value is JSONCOMPAT_MISSING:
            return value
        raise TypeError("expected JSONCOMPAT_MISSING sentinel")
    if annotation is str:
        if isinstance(value, str):
            return value
        raise TypeError(f"expected str, got {type(value).__name__}")
    if annotation is int:
        if isinstance(value, int) and not isinstance(value, bool):
            return value
        raise TypeError(f"expected int, got {type(value).__name__}")
    if annotation is float:
        if isinstance(value, (int, float)) and not isinstance(value, bool):
            return value
        raise TypeError(f"expected number, got {type(value).__name__}")
    if annotation is bool:
        if isinstance(value, bool):
            return value
        raise TypeError(f"expected bool, got {type(value).__name__}")
    if annotation is type(None):
        if value is None:
            return None
        raise TypeError(f"expected null, got {type(value).__name__}")
    if isinstance(annotation, type) and issubclass(annotation, DataclassModel):
        return annotation.jsoncompat_from_validated(value)

    origin = get_origin(annotation)
    if origin is list:
        args = get_args(annotation)
        item_annotation = args[0] if args else Any
        if not isinstance(value, list):
            raise TypeError(f"expected list, got {type(value).__name__}")
        value_items = cast(list[Any], value)
        return [
            _jsoncompat_construct_value(item_annotation, item)
            for item in value_items
        ]
    if origin in {types.UnionType, Union}:
        return _jsoncompat_construct_union(get_args(annotation), value)
    if origin is Literal:
        if value in get_args(annotation):
            return value
        raise TypeError(f"expected one of {get_args(annotation)!r}, got {value!r}")

    return value


def _jsoncompat_construct_union(branches: tuple[Any, ...], value: Any) -> Any:
    instance_json: str | None = None
    for branch in branches:
        if branch is JsoncompatMissingType and value is JSONCOMPAT_MISSING:
            return JSONCOMPAT_MISSING
        if isinstance(branch, type) and issubclass(branch, DataclassModel):
            schema_json = _jsoncompat_schema_for(branch)
            if schema_json is None:
                continue
            if instance_json is None:
                instance_json = json.dumps(value, separators=(",", ":"), sort_keys=True)
            if not is_valid(schema_json, instance_json):
                continue
            return branch.jsoncompat_from_validated(value)
        try:
            return _jsoncompat_construct_value(branch, value)
        except (TypeError, ValueError):
            continue
    raise TypeError(f"value {value!r} does not match any union branch")


def _jsoncompat_new_unchecked[JSONCOMPAT_MODEL_T: DataclassModel](
    model_type: type[JSONCOMPAT_MODEL_T],
    values: dict[str, Any],
) -> JSONCOMPAT_MODEL_T:
    instance = object.__new__(model_type)
    for field in _jsoncompat_dataclass_fields(model_type):
        if field.name in values:
            value = values[field.name]
        elif field.default is not dataclasses.MISSING:
            value = field.default
        elif field.default_factory is not dataclasses.MISSING:
            value = field.default_factory()
        else:
            raise TypeError(
                f"{model_type.__name__} is missing required field {field.name}"
            )
        object.__setattr__(instance, field.name, value)
    return instance


def _jsoncompat_serialize_value(value: Any) -> Any:
    if value is JSONCOMPAT_MISSING:
        return JSONCOMPAT_MISSING
    if isinstance(value, DataclassModel):
        return value.jsoncompat_to_json_unchecked()
    if isinstance(value, list):
        value_items = cast(list[Any], value)
        return [_jsoncompat_serialize_value(item) for item in value_items]
    if isinstance(value, dict):
        value_object = cast(dict[str, Any], value)
        return {
            key: _jsoncompat_serialize_value(item)
            for key, item in value_object.items()
        }
    return value


def _jsoncompat_empty_extra() -> dict[str, Any]:
    return {}


def _jsoncompat_dataclass_fields(
    model_or_instance: object,
) -> tuple[dataclasses.Field[Any], ...]:
    return dataclasses.fields(cast(Any, model_or_instance))

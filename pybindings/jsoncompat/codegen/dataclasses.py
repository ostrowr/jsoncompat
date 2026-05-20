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
    TypeVar,
    Union,
    cast,
    get_args,
    get_origin,
    get_type_hints,
)

from jsoncompat import validator_for


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
JSONCOMPAT_ADDITIONAL_T = TypeVar("JSONCOMPAT_ADDITIONAL_T")


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
        _jsoncompat_validate_model_instance(self)
        value = self.jsoncompat_to_json_unchecked()
        if not _jsoncompat_validator_for(type(self)).is_valid_value(value):
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
        if not _jsoncompat_validator_for(cls).is_valid_value(value):
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
        if not _jsoncompat_validator_for(type(self)).is_valid_value(value):
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
def _jsoncompat_validator_for(model_type: type[DataclassModel]) -> Any:
    schema_json = _jsoncompat_schema_for(model_type)
    if schema_json is None:
        raise TypeError(f"{model_type.__name__} is missing __jsoncompat_schema__")
    return validator_for(schema_json)


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
    value_annotation = _jsoncompat_extra_value_annotation(annotation)
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
        if isinstance(value, float) and value.is_integer():
            return int(value)
        raise TypeError(f"expected int, got {type(value).__name__}")
    if annotation is float:
        if isinstance(value, (int, float)) and not isinstance(value, bool):
            return value
        raise TypeError(f"expected number, got {type(value).__name__}")
    if annotation is bool:
        if isinstance(value, bool):
            return value
        raise TypeError(f"expected bool, got {type(value).__name__}")
    if annotation is None or annotation is type(None):
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
    if origin is dict:
        key_annotation, value_annotation = _jsoncompat_dict_annotations(annotation)
        if not isinstance(value, dict):
            raise TypeError(f"expected dict, got {type(value).__name__}")
        value_object = cast(dict[Any, Any], value)
        return {
            _jsoncompat_construct_value(key_annotation, key): _jsoncompat_construct_value(
                value_annotation,
                item,
            )
            for key, item in value_object.items()
        }
    if origin in {types.UnionType, Union}:
        return _jsoncompat_construct_union(get_args(annotation), value)
    if origin is Literal:
        if value in get_args(annotation):
            return value
        raise TypeError(f"expected one of {get_args(annotation)!r}, got {value!r}")

    raise TypeError(f"unsupported runtime annotation {annotation!r}")


def _jsoncompat_construct_union(branches: tuple[Any, ...], value: Any) -> Any:
    rejected_dataclass_branches: list[type[DataclassModel]] = []
    for branch in branches:
        if branch is JsoncompatMissingType and value is JSONCOMPAT_MISSING:
            return JSONCOMPAT_MISSING
        if isinstance(branch, type) and issubclass(branch, DataclassModel):
            schema_json = _jsoncompat_schema_for(branch)
            if schema_json is None:
                continue
            if not _jsoncompat_validator_for(branch).is_valid_value(value):
                rejected_dataclass_branches.append(branch)
                continue
            return branch.jsoncompat_from_validated(value)
        try:
            return _jsoncompat_construct_value(branch, value)
        except (TypeError, ValueError):
            continue

    # Canonicalized helper branches can be narrower than the already-validated
    # containing schema, so keep a structural fallback before rejecting.
    for branch in rejected_dataclass_branches:
        try:
            return branch.jsoncompat_from_validated(value)
        except (TypeError, ValueError):
            continue

    raise TypeError(f"value {value!r} does not match any union branch")


def _jsoncompat_validate_model_instance(model: DataclassModel) -> None:
    if isinstance(model, DataclassRootModel):
        _jsoncompat_validate_python_value(
            _jsoncompat_root_annotation_for(type(model)),
            model.root,
        )
        return

    object_spec = _jsoncompat_object_spec_for(type(model))
    for field_spec in object_spec.fields:
        field_value = getattr(model, field_spec.py_name)
        if field_value is JSONCOMPAT_MISSING:
            if field_spec.omittable:
                continue
            raise TypeError(
                f"{type(model).__name__}.{field_spec.py_name} cannot be JSONCOMPAT_MISSING"
            )
        try:
            _jsoncompat_validate_python_value(field_spec.annotation, field_value)
        except TypeError as error:
            raise TypeError(
                f"{type(model).__name__}.{field_spec.py_name}: {error}"
            ) from None

    if object_spec.extra_annotation is None:
        return

    extra = getattr(model, JSONCOMPAT_EXTRA_FIELD)
    if not isinstance(extra, dict):
        raise TypeError(
            f"{type(model).__name__}.{JSONCOMPAT_EXTRA_FIELD} expected dict, "
            f"got {type(extra).__name__}"
        )

    extra_values = cast(dict[Any, Any], extra)
    value_annotation = _jsoncompat_extra_value_annotation(object_spec.extra_annotation)
    for json_name, item in extra_values.items():
        if not isinstance(json_name, str):
            raise TypeError(
                f"{type(model).__name__}.{JSONCOMPAT_EXTRA_FIELD} keys must be str"
            )
        try:
            _jsoncompat_validate_python_value(value_annotation, item)
        except TypeError as error:
            raise TypeError(
                f"{type(model).__name__}.{JSONCOMPAT_EXTRA_FIELD}[{json_name!r}]: {error}"
            ) from None


def _jsoncompat_extra_value_annotation(annotation: Any) -> Any:
    origin = get_origin(annotation)
    if origin is dict:
        args = get_args(annotation)
        return args[1] if len(args) == 2 else Any
    return Any


def _jsoncompat_dict_annotations(annotation: Any) -> tuple[Any, Any]:
    args = get_args(annotation)
    if len(args) == 2:
        return args[0], args[1]
    return Any, Any


def _jsoncompat_validate_python_value(annotation: Any, value: Any) -> None:
    if annotation is Any:
        return
    if annotation is JsoncompatMissingType:
        if value is JSONCOMPAT_MISSING:
            return
        raise TypeError("expected JSONCOMPAT_MISSING sentinel")
    if annotation is str:
        if isinstance(value, str):
            return
        raise TypeError(f"expected str, got {type(value).__name__}")
    if annotation is int:
        if isinstance(value, int) and not isinstance(value, bool):
            return
        raise TypeError(f"expected int, got {type(value).__name__}")
    if annotation is float:
        if isinstance(value, (int, float)) and not isinstance(value, bool):
            return
        raise TypeError(f"expected number, got {type(value).__name__}")
    if annotation is bool:
        if isinstance(value, bool):
            return
        raise TypeError(f"expected bool, got {type(value).__name__}")
    if annotation is None or annotation is type(None):
        if value is None:
            return
        raise TypeError(f"expected null, got {type(value).__name__}")
    if isinstance(annotation, type) and issubclass(annotation, DataclassModel):
        if isinstance(value, annotation):
            return
        raise TypeError(
            f"expected {annotation.__name__}, got {type(value).__name__}"
        )

    origin = get_origin(annotation)
    if origin is list:
        args = get_args(annotation)
        item_annotation = args[0] if args else Any
        if not isinstance(value, list):
            raise TypeError(f"expected list, got {type(value).__name__}")
        value_items = cast(list[Any], value)
        for item in value_items:
            _jsoncompat_validate_python_value(item_annotation, item)
        return
    if origin is dict:
        key_annotation, value_annotation = _jsoncompat_dict_annotations(annotation)
        if not isinstance(value, dict):
            raise TypeError(f"expected dict, got {type(value).__name__}")
        value_object = cast(dict[Any, Any], value)
        for key, item in value_object.items():
            _jsoncompat_validate_python_value(key_annotation, key)
            _jsoncompat_validate_python_value(value_annotation, item)
        return
    if origin in {types.UnionType, Union}:
        for branch in get_args(annotation):
            try:
                _jsoncompat_validate_python_value(branch, value)
            except TypeError:
                continue
            return
        raise TypeError(f"value {value!r} does not match any union branch")
    if origin is Literal:
        if value in get_args(annotation):
            return
        raise TypeError(f"expected one of {get_args(annotation)!r}, got {value!r}")

    raise TypeError(f"unsupported runtime annotation {annotation!r}")


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

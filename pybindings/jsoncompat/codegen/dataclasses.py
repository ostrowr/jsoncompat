from __future__ import annotations

import dataclasses
import functools
import types
from collections.abc import Mapping
from typing import (
    Any,
    Callable,
    ClassVar,
    Literal,
    NoReturn,
    TypeVar,
    Union,
    cast,
    get_args,
    get_origin,
    get_type_hints,
    overload,
)

from jsoncompat import JsonValue, ModelConverter, compile_model_converter, validator_for

from .serialization import (
    SerializationFormat,
    deserialize_value,
    serialize_value,
)


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
    "SerializationFormat",
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
type _JsoncompatConstructor = Callable[[Any, bool], Any]
type _JsoncompatSerializer = Callable[[Any], Any]
type _JsoncompatPythonValidator = Callable[[Any], None]
type _JsoncompatModelConstructor = Callable[[Any, bool], DataclassModel]
type _JsoncompatModelSerializer = Callable[[DataclassModel], Any]


@dataclasses.dataclass(frozen=True, slots=True)
class _JsoncompatFieldSpec:
    py_name: str
    json_name: str
    annotation: Any
    omittable: bool
    constructor: _JsoncompatConstructor
    validated_constructor: _JsoncompatConstructor
    serializer: _JsoncompatSerializer
    python_validator: _JsoncompatPythonValidator


@dataclasses.dataclass(frozen=True, slots=True)
class _JsoncompatObjectSpec:
    fields: tuple[_JsoncompatFieldSpec, ...]
    known_json_names: frozenset[str]
    extra_annotation: Any | None
    extra_constructor: _JsoncompatConstructor | None
    extra_validated_constructor: _JsoncompatConstructor | None
    extra_serializer: _JsoncompatSerializer | None
    extra_python_validator: _JsoncompatPythonValidator | None


@dataclasses.dataclass(frozen=True, slots=True)
class _JsoncompatDiscriminatorPlan:
    json_name: str
    branches_by_literal: Mapping[tuple[str, Any], type[DataclassModel]]


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


@dataclasses.dataclass(frozen=True, slots=True, kw_only=True)
class DataclassModel:
    skip_validation: dataclasses.InitVar[bool] = False

    __jsoncompat_schema__: ClassVar[str]

    def __post_init__(self, skip_validation: bool) -> None:
        schema_json = _jsoncompat_schema_for(type(self))
        if schema_json is None:
            return
        _jsoncompat_validate_model_instance(self)
        if skip_validation:
            return
        value = self.jsoncompat_to_value_unchecked()
        if not _jsoncompat_validator_for(type(self))._is_valid_borrowed_value(value):
            raise ValueError(
                f"{type(self).__name__} instance does not satisfy its JSON Schema"
            )

    @classmethod
    def from_value[JSONCOMPAT_MODEL_T: DataclassModel](
        cls: type[JSONCOMPAT_MODEL_T],
        value: JsonValue,
        *,
        skip_validation: bool = False,
    ) -> JSONCOMPAT_MODEL_T:
        schema_json = _jsoncompat_schema_for(cls)
        if schema_json is None:
            raise TypeError(f"{cls.__name__} is missing __jsoncompat_schema__")
        if not skip_validation:
            validator = _jsoncompat_validator_for(cls)
            native_converter = _jsoncompat_native_converter_for(cls)
            if native_converter is not None:
                converted = validator.construct_value(value, native_converter)
                if converted is None:
                    raise ValueError(f"value does not satisfy {cls.__name__} schema")
                return cast(JSONCOMPAT_MODEL_T, converted)
            if not validator.is_valid_value(value):
                raise ValueError(f"value does not satisfy {cls.__name__} schema")
        return cls.jsoncompat_from_validated(
            value,
            _validate_union_branches=not skip_validation,
        )

    @classmethod
    def deserialize[JSONCOMPAT_MODEL_T: DataclassModel](
        cls: type[JSONCOMPAT_MODEL_T],
        payload: str | bytes,
        *,
        format: SerializationFormat = SerializationFormat.JSON,
        skip_validation: bool = False,
    ) -> JSONCOMPAT_MODEL_T:
        selected_format = SerializationFormat(format)
        if selected_format is SerializationFormat.JSON:
            schema_json = _jsoncompat_schema_for(cls)
            if schema_json is None:
                raise TypeError(f"{cls.__name__} is missing __jsoncompat_schema__")
            native_converter = _jsoncompat_native_converter_for(cls)
            if native_converter is not None:
                converted = _jsoncompat_validator_for(cls).construct_json(
                    payload,
                    native_converter,
                    not skip_validation,
                )
                if converted is None:
                    raise ValueError(f"value does not satisfy {cls.__name__} schema")
                return cast(JSONCOMPAT_MODEL_T, converted)
        if selected_format is SerializationFormat.JSON and not skip_validation:
            is_valid, value = _jsoncompat_validator_for(cls).parse_json(payload)
            if not is_valid:
                raise ValueError(f"value does not satisfy {cls.__name__} schema")
            return cls.jsoncompat_from_validated(
                value,
                _validate_union_branches=True,
            )
        return cls.from_value(
            deserialize_value(payload, format=selected_format),
            skip_validation=skip_validation,
        )

    def to_value(self, *, skip_validation: bool = False) -> JsonValue:
        value = self.jsoncompat_to_value_unchecked()
        schema_json = _jsoncompat_schema_for(type(self))
        if schema_json is None:
            raise TypeError(f"{type(self).__name__} is missing __jsoncompat_schema__")
        if not skip_validation and not _jsoncompat_validator_for(
            type(self)
        )._is_valid_borrowed_value(value):
            raise ValueError(
                f"{type(self).__name__} instance does not satisfy its JSON Schema"
            )
        return cast(JsonValue, value)

    @overload
    def serialize(
        self,
        *,
        format: Literal[SerializationFormat.JSON] = SerializationFormat.JSON,
        skip_validation: bool = False,
    ) -> str: ...

    @overload
    def serialize(
        self,
        *,
        format: Literal[SerializationFormat.YAML],
        skip_validation: bool = False,
    ) -> str: ...

    @overload
    def serialize(
        self,
        *,
        format: Literal[SerializationFormat.MSGPACK],
        skip_validation: bool = False,
    ) -> bytes: ...

    @overload
    def serialize(
        self,
        *,
        format: SerializationFormat,
        skip_validation: bool = False,
    ) -> str | bytes: ...

    def serialize(
        self,
        *,
        format: SerializationFormat = SerializationFormat.JSON,
        skip_validation: bool = False,
    ) -> str | bytes:
        selected_format = SerializationFormat(format)
        if selected_format is SerializationFormat.JSON:
            schema_json = _jsoncompat_schema_for(type(self))
            native_converter = _jsoncompat_native_converter_for(type(self))
            if schema_json is not None and native_converter is not None:
                validator = _jsoncompat_validator_for(type(self))
                try:
                    encoded = validator.serialize_model(
                        self,
                        native_converter,
                        not skip_validation,
                    )
                except OverflowError:
                    pass
                else:
                    if encoded is None:
                        raise ValueError(
                            f"{type(self).__name__} instance does not satisfy its JSON Schema"
                        )
                    return encoded

            value = self.jsoncompat_to_value_unchecked()
            if skip_validation:
                return serialize_value(cast(JsonValue, value))
            if schema_json is None:
                raise TypeError(
                    f"{type(self).__name__} is missing __jsoncompat_schema__"
                )
            validator = _jsoncompat_validator_for(type(self))
            try:
                encoded = validator.serialize_json(cast(JsonValue, value))
            except OverflowError:
                if not validator._is_valid_borrowed_value(cast(JsonValue, value)):
                    raise ValueError(
                        f"{type(self).__name__} instance does not satisfy its JSON Schema"
                    ) from None
                return serialize_value(cast(JsonValue, value))
            if encoded is None:
                raise ValueError(
                    f"{type(self).__name__} instance does not satisfy its JSON Schema"
                )
            return encoded
        return serialize_value(
            self.to_value(skip_validation=skip_validation),
            format=selected_format,
        )

    @classmethod
    def jsoncompat_from_validated[JSONCOMPAT_MODEL_T: DataclassModel](
        cls: type[JSONCOMPAT_MODEL_T],
        value: Any,
        *,
        _validate_union_branches: bool = True,
    ) -> JSONCOMPAT_MODEL_T:
        native_converter = _jsoncompat_native_converter_for(cls)
        if native_converter is not None:
            return cast(
                JSONCOMPAT_MODEL_T,
                native_converter.construct(value, _validate_union_branches),
            )
        constructor = _jsoncompat_object_constructor_for(cls)
        return cast(
            JSONCOMPAT_MODEL_T,
            constructor(value, _validate_union_branches),
        )

    def jsoncompat_to_value_unchecked(self) -> Any:
        return _jsoncompat_object_serializer_for(type(self))(self)


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
        *,
        _validate_union_branches: bool = True,
    ) -> JSONCOMPAT_ROOT_MODEL_T:
        native_converter = _jsoncompat_native_converter_for(cls)
        if native_converter is not None:
            return cast(
                JSONCOMPAT_ROOT_MODEL_T,
                native_converter.construct(value, _validate_union_branches),
            )
        return _jsoncompat_new_unchecked(
            cls,
            {
                "root": _jsoncompat_construct_value(
                    _jsoncompat_root_annotation_for(cls),
                    value,
                    validate_union_branches=_validate_union_branches,
                )
            },
        )

    def jsoncompat_to_value_unchecked(self) -> Any:
        return _jsoncompat_serializer_for(_jsoncompat_root_annotation_for(type(self)))(
            self.root
        )


class ReaderDataclassModel(DataclassModel):
    __slots__ = ()

    def to_value(self, *, skip_validation: bool = False) -> NoReturn:
        _ = skip_validation
        raise TypeError("Reader dataclasses do not support serialization")

    def serialize(
        self,
        *,
        format: SerializationFormat = SerializationFormat.JSON,
        skip_validation: bool = False,
    ) -> NoReturn:
        _ = (format, skip_validation)
        raise TypeError("Reader dataclasses do not support serialization")


class ReaderDataclassRootModel(DataclassRootModel):
    __slots__ = ()

    def to_value(self, *, skip_validation: bool = False) -> NoReturn:
        _ = skip_validation
        raise TypeError("Reader dataclasses do not support serialization")

    def serialize(
        self,
        *,
        format: SerializationFormat = SerializationFormat.JSON,
        skip_validation: bool = False,
    ) -> NoReturn:
        _ = (format, skip_validation)
        raise TypeError("Reader dataclasses do not support serialization")


class WriterDataclassModel(DataclassModel):
    __slots__ = ()

    @classmethod
    def from_value(
        cls,
        value: JsonValue,
        *,
        skip_validation: bool = False,
    ) -> NoReturn:
        _ = (value, skip_validation)
        raise TypeError("Writer dataclasses do not support deserialization")

    @classmethod
    def deserialize(
        cls,
        payload: str | bytes,
        *,
        format: SerializationFormat = SerializationFormat.JSON,
        skip_validation: bool = False,
    ) -> NoReturn:
        _ = (payload, format, skip_validation)
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
                constructor=_jsoncompat_constructor_for(annotation),
                validated_constructor=_jsoncompat_validated_constructor_for(annotation),
                serializer=_jsoncompat_serializer_for(annotation),
                python_validator=_jsoncompat_python_validator_for(annotation),
            )
        )
        known_json_names.add(json_name)

    extra_value_annotation = (
        _jsoncompat_extra_value_annotation(extra_annotation)
        if extra_annotation is not None
        else None
    )
    return _JsoncompatObjectSpec(
        fields=tuple(fields),
        known_json_names=frozenset(known_json_names),
        extra_annotation=extra_annotation,
        extra_constructor=(
            _jsoncompat_constructor_for(extra_value_annotation)
            if extra_annotation is not None
            else None
        ),
        extra_validated_constructor=(
            _jsoncompat_validated_constructor_for(extra_value_annotation)
            if extra_annotation is not None
            else None
        ),
        extra_serializer=(
            _jsoncompat_serializer_for(extra_value_annotation)
            if extra_annotation is not None
            else None
        ),
        extra_python_validator=(
            _jsoncompat_python_validator_for(extra_value_annotation)
            if extra_annotation is not None
            else None
        ),
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


class _JsoncompatNativePlanUnsupported(Exception):
    pass


class _JsoncompatNativePlanBuilder:
    def __init__(self) -> None:
        self.nodes: list[tuple[Any, ...] | None] = []
        self.node_ids: dict[Any, int] = {}

    def add(self, annotation: Any) -> int:
        existing = self.node_ids.get(annotation)
        if existing is not None:
            return existing
        node_id = len(self.nodes)
        self.node_ids[annotation] = node_id
        self.nodes.append(None)
        self.nodes[node_id] = self._descriptor(annotation)
        return node_id

    def finish(self, root: int) -> tuple[list[tuple[Any, ...]], int]:
        if any(node is None for node in self.nodes):
            raise RuntimeError("native model conversion plan has unresolved nodes")
        return cast(list[tuple[Any, ...]], self.nodes), root

    def _descriptor(self, annotation: Any) -> tuple[Any, ...]:
        if annotation is Any:
            return ("any",)
        if annotation is JsoncompatMissingType:
            return ("missing", JSONCOMPAT_MISSING)
        if annotation is str:
            return ("str",)
        if annotation is int:
            return ("int",)
        if annotation is float:
            return ("float",)
        if annotation is bool:
            return ("bool",)
        if annotation is None or annotation is type(None):
            return ("null",)
        if isinstance(annotation, type) and issubclass(annotation, DataclassModel):
            return self._model_descriptor(annotation)

        origin = get_origin(annotation)
        if origin is list:
            args = get_args(annotation)
            item_annotation = args[0] if args else Any
            return ("list", self.add(item_annotation))
        if origin is dict:
            key_annotation, value_annotation = _jsoncompat_dict_annotations(annotation)
            return (
                "dict",
                self.add(key_annotation),
                self.add(value_annotation),
            )
        if origin is Literal:
            return ("literal", get_args(annotation))
        if origin in {types.UnionType, Union}:
            return self._union_descriptor(get_args(annotation))
        raise _JsoncompatNativePlanUnsupported

    def _model_descriptor(
        self,
        model_type: type[DataclassModel],
    ) -> tuple[Any, ...]:
        if issubclass(model_type, DataclassRootModel):
            return (
                "root",
                model_type,
                self.add(_jsoncompat_root_annotation_for(model_type)),
            )

        object_spec = _jsoncompat_object_spec_for(model_type)
        fields_by_name = {
            field.name: field for field in _jsoncompat_dataclass_fields(model_type)
        }
        fields = tuple(
            (
                field_spec.json_name,
                field_spec.py_name,
                self.add(field_spec.annotation),
                _jsoncompat_missing_value_factory(
                    model_type,
                    fields_by_name[field_spec.py_name],
                    field_spec.omittable,
                ),
                JSONCOMPAT_MISSING if field_spec.omittable else None,
            )
            for field_spec in object_spec.fields
        )
        extra_value = (
            self.add(_jsoncompat_extra_value_annotation(object_spec.extra_annotation))
            if object_spec.extra_annotation is not None
            else None
        )
        return ("model", model_type, fields, extra_value)

    def _union_descriptor(self, branches: tuple[Any, ...]) -> tuple[Any, ...]:
        branch_ids = tuple(self.add(branch) for branch in branches)
        plans = _jsoncompat_discriminator_plans_for(branches)
        if plans:
            plan = plans[0]
            branches_by_value: dict[str, int] = {}
            for literal_key, branch in plan.branches_by_literal.items():
                if literal_key[0] != "str" or not isinstance(literal_key[1], str):
                    break
                branches_by_value[literal_key[1]] = self.node_ids[branch]
            else:
                return (
                    "union",
                    branch_ids,
                    plan.json_name,
                    branches_by_value,
                )

        object_like_branches = sum(
            _jsoncompat_native_branch_is_object_like(branch) for branch in branches
        )
        if object_like_branches > 1:
            raise _JsoncompatNativePlanUnsupported
        return ("union", branch_ids, None, None)


def _jsoncompat_native_branch_is_object_like(annotation: Any) -> bool:
    if isinstance(annotation, type) and issubclass(annotation, DataclassModel):
        return True
    return get_origin(annotation) is dict


@functools.lru_cache(maxsize=None)
def _jsoncompat_native_converter_for(
    model_type: type[DataclassModel],
) -> ModelConverter | None:
    builder = _JsoncompatNativePlanBuilder()
    try:
        root = builder.add(model_type)
    except _JsoncompatNativePlanUnsupported:
        return None
    descriptors, root = builder.finish(root)
    return compile_model_converter(descriptors, root)


@functools.lru_cache(maxsize=None)
def _jsoncompat_object_constructor_for(
    model_type: type[DataclassModel],
) -> _JsoncompatModelConstructor:
    object_spec = _jsoncompat_object_spec_for(model_type)
    fields_by_name = {
        field.name: field for field in _jsoncompat_dataclass_fields(model_type)
    }
    missing_factories = tuple(
        _jsoncompat_missing_value_factory(
            model_type,
            fields_by_name[field_spec.py_name],
            field_spec.omittable,
        )
        for field_spec in object_spec.fields
    )
    specialized_namespace: dict[str, Any] = {}

    lines = [
        "def construct(value, validate_union_branches):",
        "    if not isinstance(value, dict):",
        "        raise TypeError(_object_error)",
        "    instance = _new(_model_type)",
        "    if validate_union_branches:",
    ]
    for index, field_spec in enumerate(object_spec.fields):
        json_name = repr(field_spec.json_name)
        py_name = repr(field_spec.py_name)
        raw_name = f"raw_{index}"
        discriminated_list = _jsoncompat_validated_discriminated_list_plan(
            field_spec.annotation
        )
        if discriminated_list is not None:
            discriminator_name, branches_by_value = discriminated_list
            models_name = f"_union_models_{index}"
            constructors_name = f"_union_constructors_{index}"
            converted_name = f"converted_{index}"
            item_name = f"item_{index}"
            tag_name = f"tag_{index}"
            constructor_name = f"constructor_{index}"
            branch_name = f"branch_{index}"
            specialized_namespace[models_name] = branches_by_value
            specialized_namespace[constructors_name] = {}
            lines.extend(
                [
                    f"        if {json_name} in value:",
                    f"            {raw_name} = value[{json_name}]",
                    f"            {converted_name} = []",
                    f"            for {item_name} in {raw_name}:",
                    f"                {tag_name} = {item_name}[{discriminator_name!r}]",
                    f"                {constructor_name} = {constructors_name}.get({tag_name})",
                    f"                if {constructor_name} is None:",
                    f"                    {branch_name} = {models_name}[{tag_name}]",
                    f"                    {constructor_name} = (",
                    f"                        construct if {branch_name} is _model_type",
                    f"                        else _object_constructor_for({branch_name})",
                    "                    )",
                    f"                    {constructors_name}[{tag_name}] = {constructor_name}",
                    f"                {converted_name}.append({constructor_name}({item_name}, True))",
                    f"            _setattr(instance, {py_name}, {converted_name})",
                    "        else:",
                    "            _setattr("
                    f"instance, {py_name}, _missing_factories[{index}]()"
                    ")",
                ]
            )
            continue
        converted = _jsoncompat_validated_inline_expression(
            field_spec.annotation,
            raw_name,
            index,
        )
        lines.extend(
            [
                f"        if {json_name} in value:",
                f"            {raw_name} = value[{json_name}]",
                f"            _setattr(instance, {py_name}, {converted})",
                "        else:",
                "            _setattr("
                f"instance, {py_name}, _missing_factories[{index}]()"
                ")",
            ]
        )
    if object_spec.extra_constructor is not None:
        lines.extend(
            [
                "        extra = {}",
                "        for key, item in value.items():",
                "            if key not in _known_json_names:",
                "                extra[key] = _extra_validated_constructor(item, True)",
                f"        _setattr(instance, {JSONCOMPAT_EXTRA_FIELD!r}, extra)",
            ]
        )
    lines.extend(
        [
            "        return instance",
        ]
    )
    for index, field_spec in enumerate(object_spec.fields):
        json_name = repr(field_spec.json_name)
        py_name = repr(field_spec.py_name)
        lines.extend(
            [
                f"    if {json_name} in value:",
                "        _setattr("
                f"instance, {py_name}, "
                f"_constructors[{index}](value[{json_name}], False)"
                ")",
                "    else:",
                f"        _setattr(instance, {py_name}, _missing_factories[{index}]())",
            ]
        )
    if object_spec.extra_constructor is not None:
        lines.extend(
            [
                "    extra = {}",
                "    for key, item in value.items():",
                "        if key not in _known_json_names:",
                "            extra[key] = _extra_constructor(item, False)",
                f"    _setattr(instance, {JSONCOMPAT_EXTRA_FIELD!r}, extra)",
            ]
        )
    lines.append("    return instance")

    namespace: dict[str, Any] = {
        "_constructors": tuple(
            field_spec.constructor for field_spec in object_spec.fields
        ),
        "_validated_constructors": tuple(
            field_spec.validated_constructor for field_spec in object_spec.fields
        ),
        "_extra_constructor": object_spec.extra_constructor,
        "_extra_validated_constructor": object_spec.extra_validated_constructor,
        "_known_json_names": object_spec.known_json_names,
        "_missing_factories": missing_factories,
        "_model_type": model_type,
        "_new": object.__new__,
        "_object_error": f"{model_type.__name__} expects a JSON object",
        "_setattr": object.__setattr__,
        "_object_constructor_for": _jsoncompat_object_constructor_for,
    }
    namespace.update(specialized_namespace)
    source = "\n".join(lines)
    exec(compile(source, f"<{model_type.__name__} jsoncompat constructor>", "exec"), namespace)
    return cast(_JsoncompatModelConstructor, namespace["construct"])


def _jsoncompat_validated_inline_expression(
    annotation: Any,
    value_expression: str,
    constructor_index: int,
) -> str:
    if _jsoncompat_validated_conversion_is_identity(annotation):
        return value_expression
    if annotation is int:
        return (
            f"int({value_expression}) if isinstance({value_expression}, float) "
            f"else {value_expression}"
        )

    origin = get_origin(annotation)
    if origin is list:
        args = get_args(annotation)
        item_annotation = args[0] if args else Any
        if _jsoncompat_validated_conversion_is_identity(item_annotation):
            return f"list({value_expression})"
    if origin is dict:
        key_annotation, value_annotation = _jsoncompat_dict_annotations(annotation)
        if _jsoncompat_validated_conversion_is_identity(
            key_annotation
        ) and _jsoncompat_validated_conversion_is_identity(value_annotation):
            return f"dict({value_expression})"

    return f"_validated_constructors[{constructor_index}]({value_expression}, True)"


def _jsoncompat_validated_discriminated_list_plan(
    annotation: Any,
) -> tuple[str, Mapping[str, type[DataclassModel]]] | None:
    if get_origin(annotation) is not list:
        return None
    args = get_args(annotation)
    if len(args) != 1:
        return None
    item_annotation = args[0]
    if get_origin(item_annotation) not in {types.UnionType, Union}:
        return None
    plans = _jsoncompat_discriminator_plans_for(get_args(item_annotation))
    if not plans:
        return None
    plan = plans[0]
    branches_by_value: dict[str, type[DataclassModel]] = {}
    for literal_key, branch in plan.branches_by_literal.items():
        if literal_key[0] != "str" or not isinstance(literal_key[1], str):
            return None
        branches_by_value[literal_key[1]] = branch
    return plan.json_name, types.MappingProxyType(branches_by_value)


@functools.lru_cache(maxsize=None)
def _jsoncompat_validated_conversion_is_identity(annotation: Any) -> bool:
    if annotation in {
        Any,
        str,
        float,
        bool,
        None,
        type(None),
        JsoncompatMissingType,
    }:
        return True
    origin = get_origin(annotation)
    if origin is Literal:
        return True
    if origin in {types.UnionType, Union}:
        return all(
            _jsoncompat_validated_conversion_is_identity(branch)
            for branch in get_args(annotation)
        )
    return False


def _jsoncompat_missing_value_factory(
    model_type: type[DataclassModel],
    field: dataclasses.Field[Any],
    omittable: bool,
) -> Callable[[], Any]:
    if omittable:
        return lambda: JSONCOMPAT_MISSING
    if field.default is not dataclasses.MISSING:
        return lambda: field.default
    if field.default_factory is not dataclasses.MISSING:
        return field.default_factory

    def missing_required_field() -> NoReturn:
        raise TypeError(f"{model_type.__name__} is missing required field {field.name}")

    return missing_required_field


@functools.lru_cache(maxsize=None)
def _jsoncompat_object_serializer_for(
    model_type: type[DataclassModel],
) -> _JsoncompatModelSerializer:
    object_spec = _jsoncompat_object_spec_for(model_type)
    lines = [
        "def serialize(model):",
        "    output = {}",
    ]
    for index, field_spec in enumerate(object_spec.fields):
        py_name = field_spec.py_name
        lines.extend(
            [
                f"    value = model.{py_name}",
                "    if value is not _missing:",
                f"        output[{field_spec.json_name!r}] = _serializers[{index}](value)",
            ]
        )
    if object_spec.extra_serializer is not None:
        lines.extend(
            [
                f"    for key, item in model.{JSONCOMPAT_EXTRA_FIELD}.items():",
                "        output[key] = _extra_serializer(item)",
            ]
        )
    lines.append("    return output")

    namespace: dict[str, Any] = {
        "_extra_serializer": object_spec.extra_serializer,
        "_missing": JSONCOMPAT_MISSING,
        "_serializers": tuple(
            field_spec.serializer for field_spec in object_spec.fields
        ),
    }
    source = "\n".join(lines)
    exec(compile(source, f"<{model_type.__name__} jsoncompat serializer>", "exec"), namespace)
    return cast(_JsoncompatModelSerializer, namespace["serialize"])


def _jsoncompat_construct_value(
    annotation: Any,
    value: Any,
    *,
    validate_union_branches: bool,
) -> Any:
    constructor = (
        _jsoncompat_validated_constructor_for(annotation)
        if validate_union_branches
        else _jsoncompat_constructor_for(annotation)
    )
    return constructor(value, validate_union_branches)


@functools.lru_cache(maxsize=None)
def _jsoncompat_constructor_for(annotation: Any) -> _JsoncompatConstructor:
    if annotation is Any:
        return lambda value, validate_union_branches: value
    if annotation is JsoncompatMissingType:
        def construct_missing(value: Any, validate_union_branches: bool) -> Any:
            _ = validate_union_branches
            if value is JSONCOMPAT_MISSING:
                return value
            raise TypeError("expected JSONCOMPAT_MISSING sentinel")

        return construct_missing
    if annotation is str:
        def construct_str(value: Any, validate_union_branches: bool) -> str:
            _ = validate_union_branches
            if isinstance(value, str):
                return value
            raise TypeError(f"expected str, got {type(value).__name__}")

        return construct_str
    if annotation is int:
        def construct_int(value: Any, validate_union_branches: bool) -> int:
            _ = validate_union_branches
            if isinstance(value, int) and not isinstance(value, bool):
                return value
            if isinstance(value, float) and value.is_integer():
                return int(value)
            raise TypeError(f"expected int, got {type(value).__name__}")

        return construct_int
    if annotation is float:
        def construct_float(value: Any, validate_union_branches: bool) -> int | float:
            _ = validate_union_branches
            if isinstance(value, (int, float)) and not isinstance(value, bool):
                return value
            raise TypeError(f"expected number, got {type(value).__name__}")

        return construct_float
    if annotation is bool:
        def construct_bool(value: Any, validate_union_branches: bool) -> bool:
            _ = validate_union_branches
            if isinstance(value, bool):
                return value
            raise TypeError(f"expected bool, got {type(value).__name__}")

        return construct_bool
    if annotation is None or annotation is type(None):
        def construct_none(value: Any, validate_union_branches: bool) -> None:
            _ = validate_union_branches
            if value is None:
                return None
            raise TypeError(f"expected null, got {type(value).__name__}")

        return construct_none
    if isinstance(annotation, type) and issubclass(annotation, DataclassModel):
        model_type = annotation

        def construct_model(value: Any, validate_union_branches: bool) -> Any:
            return model_type.jsoncompat_from_validated(
                value,
                _validate_union_branches=validate_union_branches,
            )

        return construct_model

    origin = get_origin(annotation)
    if origin is list:
        args = get_args(annotation)
        item_annotation = args[0] if args else Any
        item_constructor = _jsoncompat_constructor_for(item_annotation)

        def construct_list(value: Any, validate_union_branches: bool) -> list[Any]:
            if not isinstance(value, list):
                raise TypeError(f"expected list, got {type(value).__name__}")
            value_items = cast(list[Any], value)
            return [
                item_constructor(item, validate_union_branches)
                for item in value_items
            ]

        return construct_list
    if origin is dict:
        key_annotation, value_annotation = _jsoncompat_dict_annotations(annotation)
        key_constructor = _jsoncompat_constructor_for(key_annotation)
        value_constructor = _jsoncompat_constructor_for(value_annotation)

        def construct_dict(value: Any, validate_union_branches: bool) -> dict[Any, Any]:
            if not isinstance(value, dict):
                raise TypeError(f"expected dict, got {type(value).__name__}")
            value_object = cast(dict[Any, Any], value)
            return {
                key_constructor(key, validate_union_branches): value_constructor(
                    item,
                    validate_union_branches,
                )
                for key, item in value_object.items()
            }

        return construct_dict
    if origin in {types.UnionType, Union}:
        branches = get_args(annotation)
        compiled_constructor: _JsoncompatConstructor | None = None

        def construct_union(value: Any, validate_union_branches: bool) -> Any:
            nonlocal compiled_constructor
            if compiled_constructor is None:
                compiled_constructor = _jsoncompat_compiled_union_constructor_for(
                    branches
                )
            return compiled_constructor(value, validate_union_branches)

        return construct_union
    if origin is Literal:
        literals = get_args(annotation)
        literals_by_key = _jsoncompat_literals_by_key(literals)

        def construct_literal(value: Any, validate_union_branches: bool) -> Any:
            _ = validate_union_branches
            literal_key = _jsoncompat_literal_key(value)
            literal = (
                literals_by_key.get(literal_key, _JSONCOMPAT_MISSING_TYPE_HINT)
                if literal_key is not None
                else _JSONCOMPAT_MISSING_TYPE_HINT
            )
            if literal is not _JSONCOMPAT_MISSING_TYPE_HINT:
                return literal
            raise TypeError(f"expected one of {literals!r}, got {value!r}")

        return construct_literal

    def construct_unsupported(value: Any, validate_union_branches: bool) -> NoReturn:
        _ = (value, validate_union_branches)
        raise TypeError(f"unsupported runtime annotation {annotation!r}")

    return construct_unsupported


@functools.lru_cache(maxsize=None)
def _jsoncompat_validated_constructor_for(
    annotation: Any,
) -> _JsoncompatConstructor:
    if annotation in {Any, str, float, bool, None, type(None)}:
        return lambda value, validate_union_branches: value
    if annotation is JsoncompatMissingType:
        return _jsoncompat_constructor_for(annotation)
    if annotation is int:
        return lambda value, validate_union_branches: (
            int(value) if isinstance(value, float) else value
        )
    if isinstance(annotation, type) and issubclass(annotation, DataclassModel):
        model_type = annotation

        def construct_model(value: Any, validate_union_branches: bool) -> Any:
            return model_type.jsoncompat_from_validated(
                value,
                _validate_union_branches=validate_union_branches,
            )

        return construct_model

    origin = get_origin(annotation)
    if origin is list:
        args = get_args(annotation)
        item_annotation = args[0] if args else Any
        item_constructor = _jsoncompat_validated_constructor_for(item_annotation)

        def construct_list(value: Any, validate_union_branches: bool) -> list[Any]:
            value_items = cast(list[Any], value)
            return [
                item_constructor(item, validate_union_branches)
                for item in value_items
            ]

        return construct_list
    if origin is dict:
        key_annotation, value_annotation = _jsoncompat_dict_annotations(annotation)
        key_constructor = _jsoncompat_validated_constructor_for(key_annotation)
        value_constructor = _jsoncompat_validated_constructor_for(value_annotation)

        def construct_dict(value: Any, validate_union_branches: bool) -> dict[Any, Any]:
            value_object = cast(dict[Any, Any], value)
            return {
                key_constructor(key, validate_union_branches): value_constructor(
                    item,
                    validate_union_branches,
                )
                for key, item in value_object.items()
            }

        return construct_dict
    if origin in {types.UnionType, Union}:
        branches = get_args(annotation)
        compiled_constructor: _JsoncompatConstructor | None = None

        def construct_union(value: Any, validate_union_branches: bool) -> Any:
            nonlocal compiled_constructor
            if compiled_constructor is None:
                compiled_constructor = _jsoncompat_compiled_union_constructor_for(
                    branches
                )
            return compiled_constructor(value, validate_union_branches)

        return construct_union
    if origin is Literal:
        literals = get_args(annotation)
        literals_by_key = _jsoncompat_literals_by_key(literals)

        def construct_literal(value: Any, validate_union_branches: bool) -> Any:
            _ = validate_union_branches
            literal_key = _jsoncompat_literal_key(value)
            literal = (
                literals_by_key.get(literal_key, _JSONCOMPAT_MISSING_TYPE_HINT)
                if literal_key is not None
                else _JSONCOMPAT_MISSING_TYPE_HINT
            )
            if literal is not _JSONCOMPAT_MISSING_TYPE_HINT:
                return literal
            raise TypeError(f"expected one of {literals!r}, got {value!r}")

        return construct_literal
    return _jsoncompat_constructor_for(cast(Any, annotation))


@functools.lru_cache(maxsize=None)
def _jsoncompat_compiled_union_constructor_for(
    branches: tuple[Any, ...],
) -> _JsoncompatConstructor:
    plans = _jsoncompat_discriminator_plans_for(branches)
    if not plans:
        return lambda value, validate_union_branches: _jsoncompat_construct_union(
            branches,
            value,
            validate_union_branches=validate_union_branches,
        )

    plan = plans[0]
    string_branches = {
        literal_key[1]: branch
        for literal_key, branch in plan.branches_by_literal.items()
        if literal_key[0] == "str"
    }
    only_string_literals = len(string_branches) == len(plan.branches_by_literal)
    branch_constructors: dict[
        type[DataclassModel],
        _JsoncompatModelConstructor,
    ] = {}

    def construct_discriminated_union(
        value: Any,
        validate_union_branches: bool,
    ) -> Any:
        branch: type[DataclassModel] | None = None
        if isinstance(value, dict) and plan.json_name in value:
            value_object = cast(dict[Any, Any], value)
            discriminator: Any = value_object[plan.json_name]
            if only_string_literals:
                if isinstance(discriminator, str):
                    branch = string_branches.get(discriminator)
            else:
                literal_key = _jsoncompat_literal_key(discriminator)
                if literal_key is not None:
                    branch = plan.branches_by_literal.get(literal_key)
        if branch is None:
            return _jsoncompat_construct_union(
                branches,
                value,
                validate_union_branches=validate_union_branches,
            )

        constructor = branch_constructors.get(branch)
        if constructor is None:
            constructor = _jsoncompat_object_constructor_for(branch)
            branch_constructors[branch] = constructor
        return constructor(value, validate_union_branches)

    return construct_discriminated_union


def _jsoncompat_construct_union(
    branches: tuple[Any, ...],
    value: Any,
    *,
    validate_union_branches: bool,
) -> Any:
    discriminated_branch = _jsoncompat_discriminated_branch(branches, value)
    if discriminated_branch is not None:
        return discriminated_branch.jsoncompat_from_validated(
            value,
            _validate_union_branches=validate_union_branches,
        )

    matching_branches = tuple(
        branch
        for branch in branches
        if _jsoncompat_branch_matches_value_kind(branch, value)
    )
    if len(matching_branches) == 1:
        return _jsoncompat_construct_value(
            matching_branches[0],
            value,
            validate_union_branches=validate_union_branches,
        )

    candidate_branches = matching_branches or branches

    rejected_dataclass_branches: list[type[DataclassModel]] = []
    for branch in candidate_branches:
        if branch is JsoncompatMissingType and value is JSONCOMPAT_MISSING:
            return JSONCOMPAT_MISSING
        if isinstance(branch, type) and issubclass(branch, DataclassModel):
            if validate_union_branches:
                schema_json = _jsoncompat_schema_for(branch)
                if schema_json is None:
                    continue
                if not _jsoncompat_validator_for(branch)._is_valid_borrowed_value(value):
                    rejected_dataclass_branches.append(branch)
                    continue
            try:
                return branch.jsoncompat_from_validated(
                    value,
                    _validate_union_branches=validate_union_branches,
                )
            except (TypeError, ValueError):
                if validate_union_branches:
                    raise
                continue
        try:
            return _jsoncompat_construct_value(
                branch,
                value,
                validate_union_branches=validate_union_branches,
            )
        except (TypeError, ValueError):
            continue

    # Canonicalized helper branches can be narrower than the already-validated
    # containing schema, so keep a structural fallback before rejecting.
    for branch in rejected_dataclass_branches:
        try:
            return branch.jsoncompat_from_validated(
                value,
                _validate_union_branches=validate_union_branches,
            )
        except (TypeError, ValueError):
            continue

    raise TypeError(f"value {value!r} does not match any union branch")


def _jsoncompat_discriminated_branch(
    branches: tuple[Any, ...],
    value: Any,
) -> type[DataclassModel] | None:
    if not isinstance(value, dict):
        return None
    value_object = cast(dict[Any, Any], value)

    for plan in _jsoncompat_discriminator_plans_for(branches):
        if plan.json_name not in value_object:
            continue
        literal_key = _jsoncompat_literal_key(value_object[plan.json_name])
        if literal_key is None:
            continue
        branch = plan.branches_by_literal.get(literal_key)
        if branch is not None:
            return branch

    return None


@functools.lru_cache(maxsize=None)
def _jsoncompat_discriminator_plans_for(
    branches: tuple[Any, ...],
) -> tuple[_JsoncompatDiscriminatorPlan, ...]:
    model_branches: list[type[DataclassModel]] = []
    for branch in branches:
        if isinstance(branch, type) and issubclass(branch, DataclassModel):
            model_branches.append(branch)
        elif branch not in {JsoncompatMissingType, None, type(None)}:
            return ()
    if len(model_branches) < 2:
        return ()

    branch_specs = [
        (branch, _jsoncompat_object_spec_for(branch)) for branch in model_branches
    ]
    plans: list[_JsoncompatDiscriminatorPlan] = []
    for candidate_field in branch_specs[0][1].fields:
        if candidate_field.omittable:
            continue

        branches_by_literal: dict[
            tuple[str, Any],
            type[DataclassModel] | None,
        ] = {}
        for branch, object_spec in branch_specs:
            branch_field = next(
                (
                    field_spec
                    for field_spec in object_spec.fields
                    if field_spec.json_name == candidate_field.json_name
                    and not field_spec.omittable
                ),
                None,
            )
            if branch_field is None or get_origin(branch_field.annotation) is not Literal:
                break
            for literal in get_args(branch_field.annotation):
                literal_key = _jsoncompat_literal_key(literal)
                if literal_key is None:
                    continue
                if literal_key in branches_by_literal:
                    branches_by_literal[literal_key] = None
                else:
                    branches_by_literal[literal_key] = branch
        else:
            unique_branches = {
                literal_key: branch
                for literal_key, branch in branches_by_literal.items()
                if branch is not None
            }
            if unique_branches:
                plans.append(
                    _JsoncompatDiscriminatorPlan(
                        json_name=candidate_field.json_name,
                        branches_by_literal=types.MappingProxyType(unique_branches),
                    )
                )
    return tuple(plans)


def _jsoncompat_literal_key(value: Any) -> tuple[str, Any] | None:
    if isinstance(value, bool):
        return ("bool", value)
    if isinstance(value, (int, float)):
        return ("number", value)
    if value is None:
        return ("null", None)
    try:
        hash(value)
    except TypeError:
        return None
    return (type(value).__qualname__, value)


def _jsoncompat_literals_by_key(
    literals: tuple[Any, ...],
) -> Mapping[tuple[str, Any], Any]:
    values: dict[tuple[str, Any], Any] = {}
    for literal in literals:
        literal_key = _jsoncompat_literal_key(literal)
        if literal_key is not None and literal_key not in values:
            values[literal_key] = literal
    return types.MappingProxyType(values)


def _jsoncompat_branch_matches_value_kind(branch: Any, value: Any) -> bool:
    if branch is Any:
        return True
    if branch is JsoncompatMissingType:
        return value is JSONCOMPAT_MISSING
    if branch is str:
        return isinstance(value, str)
    if branch is int:
        return (
            (isinstance(value, int) and not isinstance(value, bool))
            or (isinstance(value, float) and value.is_integer())
        )
    if branch is float:
        return isinstance(value, (int, float)) and not isinstance(value, bool)
    if branch is bool:
        return isinstance(value, bool)
    if branch is None or branch is type(None):
        return value is None
    if isinstance(branch, type) and issubclass(branch, DataclassModel):
        return isinstance(value, dict)

    origin = get_origin(branch)
    if origin is list:
        return isinstance(value, list)
    if origin is dict:
        return isinstance(value, dict)
    if origin in {types.UnionType, Union}:
        return any(
            _jsoncompat_branch_matches_value_kind(nested_branch, value)
            for nested_branch in get_args(branch)
        )
    if origin is Literal:
        return (
            _jsoncompat_literal_value(get_args(branch), value)
            is not _JSONCOMPAT_MISSING_TYPE_HINT
        )
    return True


def _jsoncompat_literal_value(literals: tuple[Any, ...], value: Any) -> Any:
    for literal in literals:
        if isinstance(literal, bool) or isinstance(value, bool):
            if type(literal) is type(value) and literal == value:
                return literal
        elif isinstance(literal, (int, float)) and isinstance(value, (int, float)):
            if literal == value:
                return literal
        elif type(literal) is type(value) and literal == value:
            return literal
    return _JSONCOMPAT_MISSING_TYPE_HINT


def _jsoncompat_validate_model_instance(model: DataclassModel) -> None:
    if isinstance(model, DataclassRootModel):
        _jsoncompat_python_validator_for(
            _jsoncompat_root_annotation_for(type(model))
        )(model.root)
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
            field_spec.python_validator(field_value)
        except TypeError as error:
            raise TypeError(
                f"{type(model).__name__}.{field_spec.py_name}: {error}"
            ) from None

    if object_spec.extra_python_validator is None:
        return

    extra = getattr(model, JSONCOMPAT_EXTRA_FIELD)
    if not isinstance(extra, dict):
        raise TypeError(
            f"{type(model).__name__}.{JSONCOMPAT_EXTRA_FIELD} expected dict, "
            f"got {type(extra).__name__}"
        )

    extra_values = cast(dict[Any, Any], extra)
    for json_name, item in extra_values.items():
        if not isinstance(json_name, str):
            raise TypeError(
                f"{type(model).__name__}.{JSONCOMPAT_EXTRA_FIELD} keys must be str"
            )
        try:
            object_spec.extra_python_validator(item)
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


@functools.lru_cache(maxsize=None)
def _jsoncompat_python_validator_for(
    annotation: Any,
) -> _JsoncompatPythonValidator:
    if annotation is Any:
        return lambda value: None
    if annotation is JsoncompatMissingType:
        def validate_missing(value: Any) -> None:
            if value is not JSONCOMPAT_MISSING:
                raise TypeError("expected JSONCOMPAT_MISSING sentinel")

        return validate_missing
    if annotation is str:
        def validate_str(value: Any) -> None:
            if not isinstance(value, str):
                raise TypeError(f"expected str, got {type(value).__name__}")

        return validate_str
    if annotation is int:
        def validate_int(value: Any) -> None:
            if not (isinstance(value, int) and not isinstance(value, bool)):
                raise TypeError(f"expected int, got {type(value).__name__}")

        return validate_int
    if annotation is float:
        def validate_float(value: Any) -> None:
            if not (
                isinstance(value, (int, float)) and not isinstance(value, bool)
            ):
                raise TypeError(f"expected number, got {type(value).__name__}")

        return validate_float
    if annotation is bool:
        def validate_bool(value: Any) -> None:
            if not isinstance(value, bool):
                raise TypeError(f"expected bool, got {type(value).__name__}")

        return validate_bool
    if annotation is None or annotation is type(None):
        def validate_none(value: Any) -> None:
            if value is not None:
                raise TypeError(f"expected null, got {type(value).__name__}")

        return validate_none
    if isinstance(annotation, type) and issubclass(annotation, DataclassModel):
        model_type = annotation

        def validate_model(value: Any) -> None:
            if not isinstance(value, model_type):
                raise TypeError(
                    f"expected {model_type.__name__}, got {type(value).__name__}"
                )

        return validate_model

    origin = get_origin(annotation)
    if origin is list:
        args = get_args(annotation)
        item_annotation = args[0] if args else Any
        item_validator = _jsoncompat_python_validator_for(item_annotation)

        def validate_list(value: Any) -> None:
            if not isinstance(value, list):
                raise TypeError(f"expected list, got {type(value).__name__}")
            value_items = cast(list[Any], value)
            for item in value_items:
                item_validator(item)

        return validate_list
    if origin is dict:
        key_annotation, value_annotation = _jsoncompat_dict_annotations(annotation)
        key_validator = _jsoncompat_python_validator_for(key_annotation)
        value_validator = _jsoncompat_python_validator_for(value_annotation)

        def validate_dict(value: Any) -> None:
            if not isinstance(value, dict):
                raise TypeError(f"expected dict, got {type(value).__name__}")
            value_object = cast(dict[Any, Any], value)
            for key, item in value_object.items():
                key_validator(key)
                value_validator(item)

        return validate_dict
    if origin in {types.UnionType, Union}:
        branch_validators = tuple(
            _jsoncompat_python_validator_for(branch) for branch in get_args(annotation)
        )

        def validate_union(value: Any) -> None:
            for branch_validator in branch_validators:
                try:
                    branch_validator(value)
                except TypeError:
                    continue
                return
            raise TypeError(f"value {value!r} does not match any union branch")

        return validate_union
    if origin is Literal:
        literals = get_args(annotation)
        literals_by_key = _jsoncompat_literals_by_key(literals)

        def validate_literal(value: Any) -> None:
            literal_key = _jsoncompat_literal_key(value)
            if literal_key is None or literal_key not in literals_by_key:
                raise TypeError(f"expected one of {literals!r}, got {value!r}")

        return validate_literal

    def validate_unsupported(value: Any) -> NoReturn:
        _ = value
        raise TypeError(f"unsupported runtime annotation {annotation!r}")

    return validate_unsupported


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


@functools.lru_cache(maxsize=None)
def _jsoncompat_serializer_for(annotation: Any) -> _JsoncompatSerializer:
    if annotation in {
        str,
        int,
        float,
        bool,
        None,
        type(None),
        JsoncompatMissingType,
    } or get_origin(annotation) is Literal:
        return lambda value: value
    if isinstance(annotation, type) and issubclass(annotation, DataclassModel):
        return lambda value: value.jsoncompat_to_value_unchecked()

    origin = get_origin(annotation)
    if origin is list:
        args = get_args(annotation)
        item_annotation = args[0] if args else Any
        item_serializer = _jsoncompat_serializer_for(item_annotation)

        def serialize_list(value: Any) -> list[Any]:
            value_items = cast(list[Any], value)
            return [item_serializer(item) for item in value_items]

        return serialize_list
    if origin is dict:
        _, value_annotation = _jsoncompat_dict_annotations(annotation)
        value_serializer = _jsoncompat_serializer_for(value_annotation)

        def serialize_dict(value: Any) -> dict[str, Any]:
            value_object = cast(dict[str, Any], value)
            return {
                key: value_serializer(item)
                for key, item in value_object.items()
            }

        return serialize_dict
    return _jsoncompat_serialize_value


def _jsoncompat_serialize_value(value: Any) -> Any:
    if value is JSONCOMPAT_MISSING:
        return JSONCOMPAT_MISSING
    if isinstance(value, DataclassModel):
        return value.jsoncompat_to_value_unchecked()
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
    model_type = (
        model_or_instance
        if isinstance(model_or_instance, type)
        else type(model_or_instance)
    )
    return _jsoncompat_dataclass_fields_for_type(model_type)


@functools.lru_cache(maxsize=None)
def _jsoncompat_dataclass_fields_for_type(
    model_type: type[Any],
) -> tuple[dataclasses.Field[Any], ...]:
    return dataclasses.fields(model_type)

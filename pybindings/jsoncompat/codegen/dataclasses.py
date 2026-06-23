from __future__ import annotations

import dataclasses
import inspect
import types
from collections.abc import Iterable, Iterator, Mapping, Sequence
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
    overload,
)

from jsoncompat import (
    JsonValue,
    ModelRuntime,
    compile_model_runtimes,
)

from .serialization import SerializationFormat, deserialize_value, serialize_value


__all__ = [
    "DataclassAdditionalModel",
    "DataclassModel",
    "DataclassRootModel",
    "FrozenDict",
    "FrozenList",
    "JSONCOMPAT_EXTRA_FIELD",
    "JSONCOMPAT_MISSING",
    "JsonValue",
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
JSONCOMPAT_RUNTIME_FIELD = "__jsoncompat_runtime__"
JSONCOMPAT_VALIDATED_FIELD = "_jsoncompat_validated"
JSONCOMPAT_JSON_NAME_METADATA = "jsoncompat_json_name"
JSONCOMPAT_MISSING_METADATA = "jsoncompat_omittable"
JSONCOMPAT_FIELD_KIND_METADATA = "jsoncompat_field_kind"
_JSONCOMPAT_PROPERTY_FIELD = "property"
_JSONCOMPAT_EXTRA_FIELD = "extra"
_JSONCOMPAT_ROOT_FIELD = "root"
_JSONCOMPAT_UNSET = object()
JSONCOMPAT_ADDITIONAL_T = TypeVar("JSONCOMPAT_ADDITIONAL_T")
_DATACLASS_MODEL_T = TypeVar("_DATACLASS_MODEL_T", bound="DataclassModel")


class JsoncompatMissingType:
    __slots__ = ()

    def __repr__(self) -> str:
        return "JSONCOMPAT_MISSING"


JSONCOMPAT_MISSING = JsoncompatMissingType()

type Omittable[T] = T | JsoncompatMissingType


class FrozenList[T](tuple[T, ...]):
    """An immutable JSON array with sequence semantics."""

    __slots__ = ()

    def __eq__(self, other: object) -> bool:
        if isinstance(other, Sequence) and not isinstance(other, (str, bytes, Mapping)):
            return tuple(self) == tuple(cast(Sequence[Any], other))
        return NotImplemented

    def __ne__(self, other: object) -> bool:
        equal = self.__eq__(other)
        if equal is NotImplemented:
            return NotImplemented
        return not equal

    __hash__ = None  # type: ignore[assignment]


class FrozenDict[K, V](Mapping[K, V]):
    """An immutable JSON object backed by an immutable tuple of pairs."""

    __slots__ = ("_items",)

    _items: tuple[tuple[K, V], ...]

    def __init__(
        self,
        values: Mapping[K, V] | Iterable[tuple[K, V]] = (),
    ) -> None:
        if hasattr(self, "_items"):
            raise TypeError("generated model JSON objects are immutable")
        if isinstance(values, Mapping):
            mapping_values = cast(Mapping[K, V], values)
            materialized: dict[K, V] = dict(mapping_values.items())
        else:
            materialized = dict(values)
        object.__setattr__(self, "_items", tuple(materialized.items()))

    def __setattr__(self, name: str, value: Any) -> NoReturn:
        _ = (name, value)
        raise TypeError("generated model JSON objects are immutable")

    def __delattr__(self, name: str) -> NoReturn:
        _ = name
        raise TypeError("generated model JSON objects are immutable")

    def __getitem__(self, key: K) -> V:
        for candidate, value in self._items:
            if candidate == key:
                return value
        raise KeyError(key)

    def __iter__(self) -> Iterator[K]:
        return (key for key, _ in self._items)

    def __len__(self) -> int:
        return len(self._items)

    def __repr__(self) -> str:
        return f"FrozenDict({dict(self._items)!r})"

    def __eq__(self, other: object) -> bool:
        if isinstance(other, Mapping):
            return dict(self._items) == dict(cast(Mapping[Any, Any], other).items())
        return NotImplemented

    __hash__ = None  # type: ignore[assignment]


class _DataclassModelMeta(type):
    @property
    def __signature__(cls) -> inspect.Signature:
        signature = inspect.signature(cls.__init__)
        return signature.replace(parameters=tuple(signature.parameters.values())[1:])

    def __call__(
        cls: type[_DATACLASS_MODEL_T],  # pyright: ignore[reportGeneralTypeIssues]
        *args: Any,
        **kwargs: Any,
    ) -> _DATACLASS_MODEL_T:
        if args:
            raise TypeError(f"{cls.__name__} is keyword-only")
        skip_validation = kwargs.pop("skip_validation", False)
        return cast(
            _DATACLASS_MODEL_T,
            _jsoncompat_runtime_for(cls).construct_kwargs(
                kwargs,
                skip_validation=skip_validation,
            ),
        )


def field(json_name: str, *, omittable: bool = False) -> Any:
    metadata = {
        JSONCOMPAT_FIELD_KIND_METADATA: _JSONCOMPAT_PROPERTY_FIELD,
        JSONCOMPAT_JSON_NAME_METADATA: json_name,
        JSONCOMPAT_MISSING_METADATA: omittable,
    }
    if omittable:
        return dataclasses.field(default=JSONCOMPAT_MISSING, metadata=metadata)
    return dataclasses.field(metadata=metadata)


def extra_field() -> Any:
    return dataclasses.field(
        default_factory=_empty_extra,
        metadata={JSONCOMPAT_FIELD_KIND_METADATA: _JSONCOMPAT_EXTRA_FIELD},
        repr=False,
    )


def _empty_extra() -> dict[str, Any]:
    return {}


def root_field() -> Any:
    return dataclasses.field(
        metadata={JSONCOMPAT_FIELD_KIND_METADATA: _JSONCOMPAT_ROOT_FIELD}
    )


@dataclasses.dataclass(frozen=True, kw_only=True)
class DataclassModel(metaclass=_DataclassModelMeta):
    """Native runtime interface shared by generated frozen dataclasses."""

    __slots__ = (JSONCOMPAT_VALIDATED_FIELD,)

    skip_validation: dataclasses.InitVar[bool] = False
    __jsoncompat_schema__: ClassVar[str]
    __jsoncompat_runtime__: ClassVar[ModelRuntime]

    def __post_init__(self, skip_validation: bool) -> NoReturn:
        _ = skip_validation
        raise RuntimeError(
            "generated dataclasses must be constructed through their native runtime"
        )

    @classmethod
    def from_value[JSONCOMPAT_MODEL_T: DataclassModel](
        cls: type[JSONCOMPAT_MODEL_T],
        value: JsonValue,
        *,
        skip_validation: bool = False,
    ) -> JSONCOMPAT_MODEL_T:
        return cast(
            JSONCOMPAT_MODEL_T,
            _jsoncompat_runtime_for(cls).from_value(
                value,
                skip_validation=skip_validation,
            ),
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
            return cast(
                JSONCOMPAT_MODEL_T,
                _jsoncompat_runtime_for(cls).deserialize(
                    payload,
                    skip_validation=skip_validation,
                ),
            )
        return cls.from_value(
            deserialize_value(payload, format=selected_format),
            skip_validation=skip_validation,
        )

    def to_value(self, *, skip_validation: bool = False) -> JsonValue:
        return _jsoncompat_runtime_for(type(self)).to_value(
            self,
            skip_validation=skip_validation,
        )

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
            return _jsoncompat_runtime_for(type(self)).serialize(
                self,
                skip_validation=skip_validation,
            )
        return serialize_value(
            self.to_value(skip_validation=skip_validation),
            format=selected_format,
        )


class DataclassAdditionalModel[JSONCOMPAT_ADDITIONAL_T](DataclassModel):
    __slots__ = ()

    __jsoncompat_extra__: Mapping[str, JSONCOMPAT_ADDITIONAL_T]

    def __class_getitem__(cls, item: Any) -> types.GenericAlias:
        # `typing`'s cached generic aliases retain generated type arguments
        # globally. A fresh PEP 585 alias keeps the same runtime base behavior
        # while allowing a discarded generated module to be collected.
        return types.GenericAlias(cls, item)

    def get_additional_property(
        self,
        json_name: str,
    ) -> JSONCOMPAT_ADDITIONAL_T | JsoncompatMissingType:
        return self.__jsoncompat_extra__.get(json_name, JSONCOMPAT_MISSING)


class DataclassRootModel(DataclassModel):
    __slots__ = ()

    root: Any


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


@dataclasses.dataclass(frozen=True, slots=True)
class _FieldSpec:
    json_name: str
    py_name: str
    annotation: Any
    omittable: bool


@dataclasses.dataclass(frozen=True, slots=True)
class _ObjectSpec:
    fields: tuple[_FieldSpec, ...]
    has_extra: bool
    extra_annotation: Any | None


@dataclasses.dataclass(frozen=True, slots=True)
class _RootSpec:
    annotation: Any


type _ModelSpec = _ObjectSpec | _RootSpec


@dataclasses.dataclass(frozen=True, slots=True)
class _DiscriminatorPlan:
    json_name: str
    branches_by_literal: Mapping[tuple[str, Any], type[DataclassModel]]


class _NativePlanUnsupported(Exception):
    pass


def _bind_generated_module(requested_type: type[DataclassModel]) -> None:
    """Compile every generated dataclass in a module on first use."""

    _validate_generated_model_class(requested_type)
    generated_init = requested_type.__dict__.get("__init__")
    namespace = getattr(generated_init, "__globals__", None)
    if not isinstance(namespace, dict):
        raise TypeError(
            f"cannot initialize {requested_type.__name__}: its generated module "
            "namespace is unavailable"
        )
    namespace = cast(dict[str, object], namespace)
    model_types = tuple(
        dict.fromkeys(
            candidate
            for candidate in namespace.values()
            if isinstance(candidate, type)
            and issubclass(candidate, DataclassModel)
            and candidate.__module__ == requested_type.__module__
        )
    )
    if requested_type not in model_types:
        raise TypeError(
            f"{requested_type.__name__} is not defined by its declared module "
            f"{requested_type.__module__!r}"
        )

    specs = {
        model_type: _inspect_generated_model(model_type, namespace)
        for model_type in model_types
    }
    builder = _NativePlanBuilder(specs)
    try:
        model_roots = [(model_type, builder.add(model_type)) for model_type in specs]
        descriptors = builder.finish()
    except _NativePlanUnsupported as error:
        raise TypeError(f"cannot compile generated models: {error}") from None
    runtimes = compile_model_runtimes(
        model_roots,
        descriptors,
        FrozenList,
        FrozenDict,
    )
    if len(runtimes) != len(model_roots):
        raise RuntimeError(
            "native model runtime compiler returned the wrong number of roots"
        )

    for (model_type, _), runtime in zip(model_roots, runtimes, strict=True):
        setattr(model_type, JSONCOMPAT_RUNTIME_FIELD, runtime)


def _inspect_generated_model(
    model_type: type[DataclassModel],
    namespace: dict[str, Any],
) -> _ModelSpec:
    _validate_generated_model_class(model_type)
    _schema_for(model_type)
    try:
        annotations = inspect.get_annotations(
            model_type,
            globals=namespace,
            locals=namespace,
            eval_str=True,
        )
    except (NameError, TypeError) as error:
        raise TypeError(
            f"cannot resolve generated annotations for {model_type.__name__}: {error}"
        ) from error

    fields: list[_FieldSpec] = []
    json_names: set[str] = set()
    extra_annotation: Any = _JSONCOMPAT_UNSET
    root_annotation: Any = _JSONCOMPAT_UNSET
    for dataclass_field in dataclasses.fields(model_type):
        if dataclass_field.name not in annotations:
            raise TypeError(
                f"generated field {model_type.__name__}.{dataclass_field.name} "
                "has no resolvable annotation"
            )
        annotation = annotations[dataclass_field.name]
        field_kind = dataclass_field.metadata.get(JSONCOMPAT_FIELD_KIND_METADATA)
        if field_kind == _JSONCOMPAT_PROPERTY_FIELD:
            json_name = dataclass_field.metadata.get(JSONCOMPAT_JSON_NAME_METADATA)
            omittable = dataclass_field.metadata.get(JSONCOMPAT_MISSING_METADATA)
            if not isinstance(json_name, str) or not isinstance(omittable, bool):
                raise TypeError(
                    f"generated field {model_type.__name__}.{dataclass_field.name} "
                    "has invalid JSON metadata"
                )
            if json_name in json_names:
                raise TypeError(
                    f"duplicate JSON field {json_name!r} in {model_type.__name__}"
                )
            json_names.add(json_name)
            if omittable:
                annotation_origin = get_origin(annotation)
                annotation_args = get_args(annotation)
                if annotation_origin is not Omittable or len(annotation_args) != 1:
                    raise TypeError(
                        f"generated omittable field {model_type.__name__}."
                        f"{dataclass_field.name} must use dc.Omittable"
                    )
                annotation = annotation_args[0]
            fields.append(
                _FieldSpec(
                    json_name,
                    dataclass_field.name,
                    annotation,
                    omittable,
                )
            )
        elif field_kind == _JSONCOMPAT_EXTRA_FIELD:
            if dataclass_field.name != JSONCOMPAT_EXTRA_FIELD:
                raise TypeError(
                    f"generated extra-properties field must be named "
                    f"{JSONCOMPAT_EXTRA_FIELD}"
                )
            if extra_annotation is not _JSONCOMPAT_UNSET:
                raise TypeError(
                    f"duplicate generated extra-properties field in {model_type.__name__}"
                )
            origin = get_origin(annotation)
            key_annotation, extra_annotation = _dict_annotations(annotation)
            if origin not in {dict, Mapping} or key_annotation is not str:
                raise TypeError(
                    f"{model_type.__name__}.{JSONCOMPAT_EXTRA_FIELD} must be a "
                    "mapping with string keys"
                )
        elif field_kind == _JSONCOMPAT_ROOT_FIELD:
            if dataclass_field.name != "root":
                raise TypeError("generated root field must be named root")
            if root_annotation is not _JSONCOMPAT_UNSET:
                raise TypeError(
                    f"duplicate generated root field in {model_type.__name__}"
                )
            root_annotation = annotation
        else:
            raise TypeError(
                f"{model_type.__name__}.{dataclass_field.name} is not a "
                "jsoncompat-generated field"
            )

    if root_annotation is not _JSONCOMPAT_UNSET:
        if fields or extra_annotation is not _JSONCOMPAT_UNSET:
            raise TypeError(
                f"generated root model {model_type.__name__} cannot declare object fields"
            )
        return _RootSpec(root_annotation)
    return _ObjectSpec(
        tuple(fields),
        extra_annotation is not _JSONCOMPAT_UNSET,
        None if extra_annotation is _JSONCOMPAT_UNSET else extra_annotation,
    )


def _validate_generated_model_class(model_type: type[DataclassModel]) -> None:
    init = model_type.__dict__.get("__init__")
    if (
        not isinstance(init, types.FunctionType)
        or init.__code__.co_filename != "<string>"
        or init.__code__.co_qualname != "__create_fn__.<locals>.__init__"
        or getattr(model_type, "__new__", None) is not object.__new__
        or getattr(model_type, "__post_init__", None)
        is not DataclassModel.__post_init__
    ):
        raise TypeError(
            f"{model_type.__name__} must be an unmodified generated frozen dataclass"
        )


class _NativePlanBuilder:
    def __init__(self, specs: Mapping[type[DataclassModel], _ModelSpec]) -> None:
        self.specs = specs
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

    def finish(self) -> list[tuple[Any, ...]]:
        if any(node is None for node in self.nodes):
            raise RuntimeError("native model conversion plan has unresolved nodes")
        return cast(list[tuple[Any, ...]], self.nodes)

    def _descriptor(self, annotation: Any) -> tuple[Any, ...]:
        if annotation is Any or annotation is JsonValue:
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
        if origin in {list, Sequence}:
            args = get_args(annotation)
            return ("list", self.add(args[0] if args else Any))
        if origin in {dict, Mapping}:
            key, value = _dict_annotations(annotation)
            return ("dict", self.add(key), self.add(value))
        if origin is Literal:
            return ("literal", get_args(annotation))
        if origin in {types.UnionType, Union}:
            return self._union_descriptor(get_args(annotation))
        raise _NativePlanUnsupported(f"unsupported annotation {annotation!r}")

    def _model_descriptor(
        self,
        model_type: type[DataclassModel],
    ) -> tuple[Any, ...]:
        spec = self.specs.get(model_type)
        if spec is None:
            raise _NativePlanUnsupported(
                f"{model_type.__name__} is not bound in this generated module"
            )
        if isinstance(spec, _RootSpec):
            return ("root", model_type, self.add(spec.annotation))

        fields = tuple(
            (
                field_spec.json_name,
                field_spec.py_name,
                self.add(field_spec.annotation),
                JSONCOMPAT_MISSING if field_spec.omittable else None,
            )
            for field_spec in spec.fields
        )
        extra_value = self.add(spec.extra_annotation) if spec.has_extra else None
        return ("model", model_type, fields, extra_value)

    def _union_descriptor(self, branches: tuple[Any, ...]) -> tuple[Any, ...]:
        branch_ids = tuple(self.add(branch) for branch in branches)
        plans = _discriminator_plans_for(self.specs, branches)
        if plans:
            plan = plans[0]
            branches_by_value: list[tuple[Any, int]] = []
            for literal_key, branch in plan.branches_by_literal.items():
                kind, value = literal_key
                if kind == "number" and (
                    isinstance(value, bool)
                    or not isinstance(value, int)
                    or not -(2**63) <= value < 2**63
                ):
                    break
                if kind not in {"str", "bool", "number", "null"}:
                    break
                branches_by_value.append((value, self.node_ids[branch]))
            else:
                return (
                    "union",
                    branch_ids,
                    plan.json_name,
                    tuple(branches_by_value),
                )

        object_like = tuple(
            branch for branch in branches if _branch_is_object_like(branch)
        )
        if len(object_like) > 1 and not all(
            isinstance(branch, type) and issubclass(branch, DataclassModel)
            for branch in object_like
        ):
            raise _NativePlanUnsupported(
                "ambiguous unions of plain mapping annotations are not generated"
            )
        return ("union", branch_ids, None, None)


def _discriminator_plans_for(
    specs: Mapping[type[DataclassModel], _ModelSpec],
    branches: tuple[Any, ...],
) -> tuple[_DiscriminatorPlan, ...]:
    model_branches: list[tuple[type[DataclassModel], _ObjectSpec]] = []
    for branch in branches:
        if isinstance(branch, type) and issubclass(branch, DataclassModel):
            spec = specs.get(branch)
            if not isinstance(spec, _ObjectSpec):
                return ()
            model_branches.append((branch, spec))
        elif branch not in {JsoncompatMissingType, None, type(None)}:
            return ()
    if len(model_branches) < 2:
        return ()

    plans: list[_DiscriminatorPlan] = []
    for candidate in model_branches[0][1].fields:
        if candidate.omittable:
            continue
        branches_by_literal: dict[tuple[str, Any], type[DataclassModel] | None] = {}
        for branch, spec in model_branches:
            branch_field = next(
                (
                    field
                    for field in spec.fields
                    if field.json_name == candidate.json_name and not field.omittable
                ),
                None,
            )
            if (
                branch_field is None
                or get_origin(branch_field.annotation) is not Literal
            ):
                break
            for literal in get_args(branch_field.annotation):
                literal_key = _literal_key(literal)
                if literal_key is None:
                    continue
                if literal_key in branches_by_literal:
                    branches_by_literal[literal_key] = None
                else:
                    branches_by_literal[literal_key] = branch
        else:
            unique = {
                literal_key: branch
                for literal_key, branch in branches_by_literal.items()
                if branch is not None
            }
            if unique:
                plans.append(_DiscriminatorPlan(candidate.json_name, unique))
    return tuple(plans)


def _schema_for(model_type: type[DataclassModel]) -> str:
    schema = getattr(model_type, JSONCOMPAT_SCHEMA_FIELD, None)
    if not isinstance(schema, str):
        raise TypeError(
            f"{model_type.__name__}.{JSONCOMPAT_SCHEMA_FIELD} must be a JSON string"
        )
    return schema


def _jsoncompat_runtime_for(model_type: type[DataclassModel]) -> ModelRuntime:
    runtime = model_type.__dict__.get(JSONCOMPAT_RUNTIME_FIELD)
    if runtime is None:
        _bind_generated_module(model_type)
        runtime = model_type.__dict__.get(JSONCOMPAT_RUNTIME_FIELD)
    if runtime is None:
        raise RuntimeError(
            f"failed to initialize generated model {model_type.__name__}"
        )
    return cast(ModelRuntime, runtime)


def _dict_annotations(annotation: Any) -> tuple[Any, Any]:
    args = get_args(annotation)
    return (args[0], args[1]) if len(args) == 2 else (Any, Any)


def _branch_is_object_like(annotation: Any) -> bool:
    if isinstance(annotation, type) and issubclass(annotation, DataclassModel):
        return True
    return get_origin(annotation) in {dict, Mapping}


def _literal_key(value: Any) -> tuple[str, Any] | None:
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

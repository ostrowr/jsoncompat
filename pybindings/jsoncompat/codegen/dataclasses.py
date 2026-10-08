from __future__ import annotations

import dataclasses
import inspect
import types
from collections.abc import Callable, Iterable, Iterator, Mapping, Sequence
from reprlib import recursive_repr
from typing import (
    Any,
    ClassVar,
    Literal,
    NoReturn,
    TypeVar,
    cast,
    dataclass_transform,
    overload,
)

from jsoncompat import (
    JSONCOMPAT_MISSING,
    JsonValue,
    JsoncompatMissingType,
    ModelRuntime,
    bind_prepared_model_runtimes,
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
JSONCOMPAT_JSON_NAME_METADATA = "jsoncompat_json_name"
JSONCOMPAT_MISSING_METADATA = "jsoncompat_omittable"
JSONCOMPAT_FIELD_KIND_METADATA = "jsoncompat_field_kind"
_JSONCOMPAT_PROPERTY_FIELD = "property"
_JSONCOMPAT_EXTRA_FIELD = "extra"
_JSONCOMPAT_ROOT_FIELD = "root"
JSONCOMPAT_ADDITIONAL_T = TypeVar("JSONCOMPAT_ADDITIONAL_T")
_DATACLASS_MODEL_T = TypeVar("_DATACLASS_MODEL_T", bound="DataclassModel")


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
    def __new__(
        mcls,
        name: str,
        bases: tuple[type, ...],
        namespace: dict[str, Any],
        **kwargs: Any,
    ) -> _DataclassModelMeta:
        inherited_generated = next(
            (
                base
                for base in bases
                if JSONCOMPAT_RUNTIME_FIELD in base.__dict__
            ),
            None,
        )
        if inherited_generated is not None:
            raise TypeError(
                f"generated model {inherited_generated.__name__} cannot be subclassed"
            )
        # Slot names are emitted by codegen. Loading attaches existing Field
        # objects and shared methods; it never resolves annotations or compiles.
        names = namespace.get("__slots__", ())
        fields: dict[str, dataclasses.Field[Any]] = {}
        for field_name in names:
            raw = namespace.get(field_name)
            if isinstance(raw, dataclasses.Field):
                field = cast(dataclasses.Field[Any], namespace.pop(field_name))
                field.name = field_name
                field.type = namespace["__annotations__"][field_name]
                field._field_type = getattr(dataclasses, "_FIELD")
                field.kw_only = True
                fields[field_name] = field
        namespace["__dataclass_fields__"] = fields
        return cast(
            _DataclassModelMeta, super().__new__(mcls, name, bases, namespace, **kwargs)
        )

    def __setattr__(cls, name: str, value: object) -> None:
        if JSONCOMPAT_RUNTIME_FIELD in cls.__dict__ and (
            name == "__slots__"
            or name in cast(tuple[str, ...], getattr(cls, "__slots__", ()))
        ):
            raise TypeError("generated slot descriptors are immutable")
        super().__setattr__(name, value)

    def __delattr__(cls, name: str) -> None:
        if JSONCOMPAT_RUNTIME_FIELD in cls.__dict__ and (
            name == "__slots__"
            or name in cast(tuple[str, ...], getattr(cls, "__slots__", ()))
        ):
            raise TypeError("generated slot descriptors are immutable")
        super().__delattr__(name)

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


_NATIVE_MISSING = JSONCOMPAT_MISSING


@overload
def field(json_name: str) -> Any: ...


@overload
def field(json_name: str, *, default: JsoncompatMissingType) -> Any: ...


def field(json_name: str, *, default: Any = dataclasses.MISSING) -> Any:
    metadata = {
        JSONCOMPAT_FIELD_KIND_METADATA: _JSONCOMPAT_PROPERTY_FIELD,
        JSONCOMPAT_JSON_NAME_METADATA: json_name,
        JSONCOMPAT_MISSING_METADATA: default is not dataclasses.MISSING,
    }
    return dataclasses.field(
        default=(
            _NATIVE_MISSING if default is not dataclasses.MISSING else dataclasses.MISSING
        ),
        metadata=metadata,
    )


def extra_field(*, default_factory: Callable[[], dict[str, Any]]) -> Any:
    return dataclasses.field(
        default_factory=default_factory,
        metadata={JSONCOMPAT_FIELD_KIND_METADATA: _JSONCOMPAT_EXTRA_FIELD},
        repr=False,
    )


def root_field() -> Any:
    return dataclasses.field(
        metadata={JSONCOMPAT_FIELD_KIND_METADATA: _JSONCOMPAT_ROOT_FIELD}
    )


@dataclass_transform(
    frozen_default=True,
    kw_only_default=True,
    field_specifiers=(field, extra_field, root_field),
)
@dataclasses.dataclass(frozen=True, kw_only=True)
class DataclassModel(metaclass=_DataclassModelMeta):
    """Native runtime interface shared by generated frozen dataclasses."""

    __slots__ = ()

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
        selected_format = (
            format
            if format is SerializationFormat.JSON
            else SerializationFormat(format)
        )
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
        selected_format = (
            format
            if format is SerializationFormat.JSON
            else SerializationFormat(format)
        )
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


# Prepared functions are shared by every model. Per-model bytecode is needed
# only for the constructor signature; construction itself uses the native plan.
EXTRA_DEFAULT = getattr(dataclasses, "_HAS_DEFAULT_FACTORY")


@recursive_repr()
def _model_repr(self: DataclassModel) -> str:
    shown = ", ".join(
        f"{field.name}={getattr(self, field.name)!r}"
        for field in dataclasses.fields(self)
        if field.repr
    )
    return self.__class__.__qualname__ + "(" + shown + ")"


def _model_eq(self: DataclassModel, other: object) -> bool:
    if type(other) is not type(self):
        return NotImplemented
    return tuple(getattr(self, name) for name in self.__slots__) == tuple(
        getattr(other, name) for name in self.__slots__
    )


def _model_hash(self: DataclassModel) -> int:
    return hash(tuple(getattr(self, name) for name in self.__slots__))


def _model_setattr(self: DataclassModel, name: str, value: object) -> NoReturn:
    raise dataclasses.FrozenInstanceError(f"cannot assign to field {name!r}")


def _model_delattr(self: DataclassModel, name: str) -> NoReturn:
    raise dataclasses.FrozenInstanceError(f"cannot delete field {name!r}")


def _model_getstate(self: DataclassModel) -> list[object]:
    return [getattr(self, name) for name in self.__slots__]


def _model_setstate(self: DataclassModel, state: Sequence[object]) -> None:
    for name, value in zip(self.__slots__, state):
        object.__setattr__(self, name, value)


_MODEL_METHODS = {
    "__repr__": _model_repr,
    "__eq__": _model_eq,
    "__hash__": _model_hash,
    "__setattr__": _model_setattr,
    "__delattr__": _model_delattr,
    "__getstate__": _model_getstate,
    "__setstate__": _model_setstate,
}


# These methods and settings are identical across all generated classes.
for _name, _method in _MODEL_METHODS.items():
    setattr(DataclassModel, _name, _method)
_params = getattr(DataclassModel, "__dataclass_params__")
for _name, _value in (("slots", True), ("weakref_slot", False)):
    if hasattr(_params, _name):
        setattr(_params, _name, _value)


class _SchemaSource:
    """Expand the original schema only when a caller explicitly requests it."""

    __slots__ = ("_compressed",)

    def __init__(self, compressed: bytes) -> None:
        self._compressed = compressed

    def __get__(self, instance: object, owner: type[DataclassModel]) -> str:
        import zlib

        schema = zlib.decompress(self._compressed).decode("utf-8")
        setattr(owner, JSONCOMPAT_SCHEMA_FIELD, schema)
        return schema


def install_model(
    model: type[DataclassModel],
    initializer: Callable[..., None],
    schema_source: bytes,
    namespace: dict[str, Any],
) -> None:
    """Bind the constructor namespace and retain compressed schema source."""
    if not isinstance(initializer, types.FunctionType):
        raise TypeError("Generated initializer must be a Python function")
    # A companion can bind several separately imported copies of the models.
    # Give each constructor the public namespace for recursive type hints.
    if initializer.__globals__ is not namespace:
        original = initializer
        initializer = types.FunctionType(original.__code__, namespace, original.__name__)
        initializer.__kwdefaults__ = original.__kwdefaults__
    fields = model.__dataclass_fields__
    initializer.__annotations__ = model.__annotations__.copy()
    initializer.__annotations__["skip_validation"] = bool
    initializer.__kwdefaults__ = {
        name: (
            _NATIVE_MISSING
            if name in fields and fields[name].metadata.get(JSONCOMPAT_MISSING_METADATA)
            else value
        )
        for name, value in (initializer.__kwdefaults__ or {}).items()
    }
    initializer.__annotations__["return"] = None
    initializer.__qualname__ = model.__qualname__ + ".__init__"
    initializer.__module__ = model.__module__
    setattr(model, "__init__", initializer)
    setattr(model, JSONCOMPAT_SCHEMA_FIELD, _SchemaSource(schema_source))


def bind_module(
    version: int,
    roots: list[tuple[type[DataclassModel], int]],
    descriptors: list[tuple[Any, ...]],
    prepared_plan: bytes,
) -> None:
    """Bind process-local class references in a precomputed model graph."""
    if version != 1:
        raise ImportError("Unsupported jsoncompat generated format; regenerate models")
    runtimes = bind_prepared_model_runtimes(
        roots, descriptors, FrozenList, FrozenDict, prepared_plan
    )
    for (model, _), runtime in zip(roots, runtimes, strict=True):
        setattr(model, JSONCOMPAT_RUNTIME_FIELD, runtime)


def _jsoncompat_runtime_for(model_type: type[DataclassModel]) -> ModelRuntime:
    runtime = model_type.__dict__.get(JSONCOMPAT_RUNTIME_FIELD)
    if runtime is None:
        raise TypeError(
            f"{model_type.__name__} has no generated runtime; regenerate models with jsoncompat codegen"
        )
    return cast(ModelRuntime, runtime)

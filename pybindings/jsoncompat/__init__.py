from __future__ import annotations

import importlib
import importlib.machinery
import importlib.util
import os
import sys
import threading
import warnings
from collections.abc import Mapping
from pathlib import Path
from typing import TYPE_CHECKING, Any, Callable, Literal, NoReturn, Protocol, cast

if TYPE_CHECKING:
    from jsoncompat._native import (
        JSONCOMPAT_MISSING as JSONCOMPAT_MISSING,
        JsoncompatMissingType as JsoncompatMissingType,
    )

RoleLiteral = Literal["serializer", "deserializer", "both"]
type JsonValue = (
    None
    | bool
    | int
    | float
    | str
    | list["JsonValue"]
    | tuple["JsonValue", ...]
    | dict[str, "JsonValue"]
)
CheckCompatFn = Callable[[str, str, RoleLiteral], bool]
GenerateValueFn = Callable[[str, int], str]
IsValidFn = Callable[[str, str], bool]
GeneratorForFn = Callable[[str], "Generator"]
ValidatorForFn = Callable[[str], "Validator"]
DeserializeJsonFn = Callable[[str | bytes], "JsonValue"]
SerializeJsonFn = Callable[["JsonValue"], str]
CompileModelRuntimesFn = Callable[
    [
        list[tuple[type[Any], int]],
        list[tuple[Any, ...]],
        type[tuple[Any, ...]],
        type[Mapping[Any, Any]],
    ],
    list["ModelRuntime"],
]


class Generator(Protocol):
    def generate_value(self, depth: int = 5) -> str: ...


class Validator(Protocol):
    def is_valid_json(self, instance_json: str) -> bool: ...
    def is_valid_value(self, instance: JsonValue) -> bool: ...
    def _is_valid_borrowed_value(self, instance: JsonValue) -> bool: ...
    def parse_json(self, payload: str | bytes) -> tuple[bool, JsonValue]: ...
    def serialize_json(self, instance: JsonValue) -> str | None: ...


class ModelRuntime(Protocol):
    def construct_kwargs(
        self,
        kwargs: dict[str, Any],
        *,
        skip_validation: bool = False,
    ) -> Any: ...

    def from_value(
        self,
        value: JsonValue,
        *,
        skip_validation: bool = False,
    ) -> Any: ...

    def deserialize(
        self,
        payload: str | bytes,
        *,
        skip_validation: bool = False,
    ) -> Any: ...

    def to_value(
        self,
        instance: Any,
        *,
        skip_validation: bool = False,
    ) -> JsonValue: ...

    def serialize(
        self,
        instance: Any,
        *,
        skip_validation: bool = False,
    ) -> str: ...


class NativeModule(Protocol):
    JSONCOMPAT_MISSING: Any
    JsoncompatMissingType: type[Any]

    def check_compat(
        self,
        old_schema_json: str,
        new_schema_json: str,
        role: RoleLiteral,
    ) -> bool: ...

    def generate_value(self, schema_json: str, depth: int) -> str: ...

    def generator_for(self, schema_json: str) -> Generator: ...

    def validator_for(self, schema_json: str) -> Validator: ...

    def deserialize_json(self, payload: str | bytes) -> JsonValue: ...

    def serialize_json(self, value: JsonValue) -> str: ...

    def compile_model_runtimes(
        self,
        model_roots: list[tuple[type[Any], int]],
        descriptors: list[tuple[Any, ...]],
        frozen_list_type: type[tuple[Any, ...]],
        frozen_dict_type: type[Mapping[Any, Any]],
    ) -> list[ModelRuntime]: ...

    def is_valid(self, schema_json: str, instance_json: str) -> bool: ...


class Role:
    SERIALIZER: RoleLiteral = "serializer"
    DESERIALIZER: RoleLiteral = "deserializer"
    BOTH: RoleLiteral = "both"


def _missing_check_compat(
    old_schema_json: str,
    new_schema_json: str,
    role: RoleLiteral,
) -> NoReturn:
    _ = (old_schema_json, new_schema_json, role)
    raise ModuleNotFoundError(
        "jsoncompat._native is unavailable. Install the built jsoncompat wheel "
        "before calling check_compat()."
    )


def _missing_generate_value(schema_json: str, depth: int) -> NoReturn:
    _ = (schema_json, depth)
    raise ModuleNotFoundError(
        "jsoncompat._native is unavailable. Install the built jsoncompat wheel "
        "before calling generate_value()."
    )


def _missing_generator_for(schema_json: str) -> NoReturn:
    _ = schema_json
    raise ModuleNotFoundError(
        "jsoncompat._native is unavailable. Install the built jsoncompat wheel "
        "before calling generator_for()."
    )


def _missing_is_valid(schema_json: str, instance_json: str) -> NoReturn:
    _ = (schema_json, instance_json)
    raise ModuleNotFoundError(
        "jsoncompat._native is unavailable. Install the built jsoncompat wheel "
        "before calling is_valid()."
    )


def _missing_validator_for(schema_json: str) -> NoReturn:
    _ = schema_json
    raise ModuleNotFoundError(
        "jsoncompat._native is unavailable. Install the built jsoncompat wheel "
        "before calling validator_for()."
    )


def _missing_deserialize_json(payload: str | bytes) -> NoReturn:
    _ = payload
    raise ModuleNotFoundError(
        "jsoncompat._native is unavailable. Install the built jsoncompat wheel "
        "before deserializing JSON."
    )


def _missing_serialize_json(value: JsonValue) -> NoReturn:
    _ = value
    raise ModuleNotFoundError(
        "jsoncompat._native is unavailable. Install the built jsoncompat wheel "
        "before serializing JSON."
    )


def _missing_compile_model_runtimes(
    model_roots: list[tuple[type[Any], int]],
    descriptors: list[tuple[Any, ...]],
    frozen_list_type: type[tuple[Any, ...]],
    frozen_dict_type: type[Mapping[Any, Any]],
) -> NoReturn:
    _ = (model_roots, descriptors, frozen_list_type, frozen_dict_type)
    raise ModuleNotFoundError(
        "jsoncompat._native is unavailable. Install the built jsoncompat wheel "
        "before constructing generated models."
    )


def _load_repo_native() -> NativeModule:
    package_dir = Path(__file__).resolve().parent
    repo_root = package_dir.parent.parent
    target_roots = [repo_root / "target"]
    if cargo_target_dir := os.environ.get("CARGO_TARGET_DIR"):
        target_roots.insert(0, Path(cargo_target_dir).expanduser())

    native_profile = os.environ.get("JSONCOMPAT_NATIVE_PROFILE")
    if native_profile is None:
        build_dirs = ("debug", "release")
    elif native_profile in {"debug", "release"}:
        build_dirs = (native_profile,)
    else:
        raise ValueError("JSONCOMPAT_NATIVE_PROFILE must be 'debug' or 'release'")

    for target_root in target_roots:
        for build_dir in build_dirs:
            for filename in (
                "libjsoncompat.dylib",
                "libjsoncompat.so",
                "jsoncompat.dll",
            ):
                native_path = target_root / build_dir / "deps" / filename
                if not native_path.exists():
                    continue

                loader = importlib.machinery.ExtensionFileLoader(
                    "jsoncompat._native",
                    str(native_path),
                )
                spec = importlib.util.spec_from_file_location(
                    "jsoncompat._native",
                    str(native_path),
                    loader=loader,
                )
                if spec is None or spec.loader is None:
                    continue

                module = importlib.util.module_from_spec(spec)
                sys.modules[spec.name] = module
                spec.loader.exec_module(module)
                return cast(NativeModule, module)

    raise ModuleNotFoundError("jsoncompat._native")


def _has_reusable_schema_api(module: object) -> bool:
    validator = getattr(module, "Validator", None)
    model_runtime = getattr(module, "ModelRuntime", None)
    return (
        hasattr(module, "generator_for")
        and hasattr(module, "validator_for")
        and hasattr(module, "deserialize_json")
        and hasattr(module, "serialize_json")
        and hasattr(module, "compile_model_runtimes")
        and hasattr(module, "JSONCOMPAT_MISSING")
        and hasattr(module, "JsoncompatMissingType")
        and hasattr(validator, "is_valid_json")
        and hasattr(validator, "is_valid_value")
        and hasattr(validator, "_is_valid_borrowed_value")
        and hasattr(validator, "parse_json")
        and hasattr(validator, "serialize_json")
        and hasattr(model_runtime, "construct_kwargs")
        and hasattr(model_runtime, "from_value")
        and hasattr(model_runtime, "deserialize")
        and hasattr(model_runtime, "to_value")
        and hasattr(model_runtime, "serialize")
    )


_force_repo_native = os.environ.get("JSONCOMPAT_NATIVE_PROFILE") is not None

try:
    if _force_repo_native:
        _native_module = _load_repo_native()
    else:
        _native_module = cast(
            NativeModule,
            importlib.import_module("jsoncompat._native"),
        )
except ModuleNotFoundError as error:
    if _force_repo_native:
        raise
    if error.name != "jsoncompat._native":
        raise
    try:
        _repo_native = _load_repo_native()
    except ModuleNotFoundError:
        _native_symbols: NativeModule | None = None
        _check_compat_native: CheckCompatFn = _missing_check_compat
        _generate_value_native: GenerateValueFn = _missing_generate_value
        _generator_for_native: GeneratorForFn = _missing_generator_for
        _validator_for_native: ValidatorForFn = _missing_validator_for
        _deserialize_json_native: DeserializeJsonFn = _missing_deserialize_json
        _serialize_json_native: SerializeJsonFn = _missing_serialize_json
        _compile_model_runtimes_native: CompileModelRuntimesFn = (
            _missing_compile_model_runtimes
        )
        _is_valid_native: IsValidFn = _missing_is_valid
    else:
        _native_symbols = _repo_native
        _check_compat_native = _repo_native.check_compat
        _generate_value_native = _repo_native.generate_value
        _generator_for_native = _repo_native.generator_for
        _validator_for_native = _repo_native.validator_for
        _deserialize_json_native = _repo_native.deserialize_json
        _serialize_json_native = _repo_native.serialize_json
        _compile_model_runtimes_native = _repo_native.compile_model_runtimes
        _is_valid_native = _repo_native.is_valid
else:
    if not _force_repo_native and not _has_reusable_schema_api(_native_module):
        _native_module = _load_repo_native()
    _native_symbols = _native_module
    _check_compat_native = _native_module.check_compat
    _generate_value_native = _native_module.generate_value
    _generator_for_native = _native_module.generator_for
    _validator_for_native = _native_module.validator_for
    _deserialize_json_native = _native_module.deserialize_json
    _serialize_json_native = _native_module.serialize_json
    _compile_model_runtimes_native = _native_module.compile_model_runtimes
    _is_valid_native = _native_module.is_valid


if not TYPE_CHECKING:
    if _native_symbols is None:
        # Ellipsis is Python's native, unforgeable singleton. It keeps source-only
        # imports usable without maintaining a second missing-value implementation;
        # installed wheels replace both names with the native jsoncompat singleton.
        JsoncompatMissingType = type(Ellipsis)
        JSONCOMPAT_MISSING = Ellipsis
    else:
        JsoncompatMissingType = _native_symbols.JsoncompatMissingType
        JSONCOMPAT_MISSING = _native_symbols.JSONCOMPAT_MISSING


def check_compat(
    old_schema_json: str,
    new_schema_json: str,
    role: RoleLiteral = "both",
) -> bool:
    check_compat_native = _check_compat_native
    return check_compat_native(old_schema_json, new_schema_json, role)


def generate_value(schema_json: str, depth: int = 5) -> str:
    warnings.warn(
        "jsoncompat.generate_value(schema_json, depth) is deprecated; "
        "use jsoncompat.generator_for(schema_json).generate_value(depth) instead.",
        DeprecationWarning,
        stacklevel=2,
    )
    generate_value_native = _generate_value_native
    return generate_value_native(schema_json, depth)


class _ThreadLocalGenerator:
    __slots__ = ("_local", "_schema_json")

    def __init__(self, schema_json: str) -> None:
        self._schema_json = schema_json
        self._local = threading.local()
        self._local.native = _generator_for_native(schema_json)

    def _native(self) -> Generator:
        native = getattr(self._local, "native", None)
        if native is None:
            native = _generator_for_native(self._schema_json)
            self._local.native = native
        return cast(Generator, native)

    def generate_value(self, depth: int = 5) -> str:
        return self._native().generate_value(depth)


class _ThreadLocalValidator:
    __slots__ = ("_local", "_schema_json")

    def __init__(self, schema_json: str) -> None:
        self._schema_json = schema_json
        self._local = threading.local()
        self._local.native = _validator_for_native(schema_json)

    def _native(self) -> Validator:
        native = getattr(self._local, "native", None)
        if native is None:
            native = _validator_for_native(self._schema_json)
            self._local.native = native
        return cast(Validator, native)

    def is_valid_json(self, instance_json: str) -> bool:
        return self._native().is_valid_json(instance_json)

    def is_valid_value(self, instance: JsonValue) -> bool:
        return self._native().is_valid_value(instance)

    def _is_valid_borrowed_value(self, instance: JsonValue) -> bool:
        return self._native()._is_valid_borrowed_value(  # pyright: ignore[reportPrivateUsage]
            instance
        )

    def parse_json(self, payload: str | bytes) -> tuple[bool, JsonValue]:
        return self._native().parse_json(payload)

    def serialize_json(self, instance: JsonValue) -> str | None:
        return self._native().serialize_json(instance)


def generator_for(schema_json: str) -> Generator:
    return _ThreadLocalGenerator(schema_json)


def validator_for(schema_json: str) -> Validator:
    return _ThreadLocalValidator(schema_json)


def deserialize_json_value(payload: str | bytes) -> JsonValue:
    deserialize_json_native = _deserialize_json_native
    return deserialize_json_native(payload)


def serialize_json_value(value: JsonValue) -> str:
    serialize_json_native = _serialize_json_native
    return serialize_json_native(value)


class _ThreadLocalModelRuntimeGroup:
    __slots__ = (
        "_descriptors",
        "_frozen_dict_type",
        "_frozen_list_type",
        "_local",
        "_model_roots",
    )

    def __init__(
        self,
        model_roots: list[tuple[type[Any], int]],
        descriptors: list[tuple[Any, ...]],
        frozen_list_type: type[tuple[Any, ...]],
        frozen_dict_type: type[Mapping[Any, Any]],
    ) -> None:
        self._model_roots = tuple(model_roots)
        self._descriptors = tuple(descriptors)
        self._frozen_list_type = frozen_list_type
        self._frozen_dict_type = frozen_dict_type
        self._local = threading.local()
        self._local.runtimes = self._compile()

    def _compile(self) -> tuple[ModelRuntime, ...]:
        return tuple(
            _compile_model_runtimes_native(
                list(self._model_roots),
                list(self._descriptors),
                self._frozen_list_type,
                self._frozen_dict_type,
            )
        )

    def runtime(self, index: int) -> ModelRuntime:
        runtimes = getattr(self._local, "runtimes", None)
        if runtimes is None:
            runtimes = self._compile()
            self._local.runtimes = runtimes
        return cast(tuple[ModelRuntime, ...], runtimes)[index]


class _ThreadLocalModelRuntime:
    __slots__ = ("_group", "_index")

    def __init__(self, group: _ThreadLocalModelRuntimeGroup, index: int) -> None:
        self._group = group
        self._index = index

    def _native(self) -> ModelRuntime:
        return self._group.runtime(self._index)

    def construct_kwargs(
        self,
        kwargs: dict[str, Any],
        *,
        skip_validation: bool = False,
    ) -> Any:
        return self._native().construct_kwargs(
            kwargs,
            skip_validation=skip_validation,
        )

    def from_value(
        self,
        value: JsonValue,
        *,
        skip_validation: bool = False,
    ) -> Any:
        return self._native().from_value(value, skip_validation=skip_validation)

    def deserialize(
        self,
        payload: str | bytes,
        *,
        skip_validation: bool = False,
    ) -> Any:
        return self._native().deserialize(payload, skip_validation=skip_validation)

    def to_value(
        self,
        instance: Any,
        *,
        skip_validation: bool = False,
    ) -> JsonValue:
        return self._native().to_value(
            instance,
            skip_validation=skip_validation,
        )

    def serialize(
        self,
        instance: Any,
        *,
        skip_validation: bool = False,
    ) -> str:
        return self._native().serialize(
            instance,
            skip_validation=skip_validation,
        )


def compile_model_runtimes(
    model_roots: list[tuple[type[Any], int]],
    descriptors: list[tuple[Any, ...]],
    frozen_list_type: type[tuple[Any, ...]],
    frozen_dict_type: type[Mapping[Any, Any]],
) -> list[ModelRuntime]:
    group = _ThreadLocalModelRuntimeGroup(
        model_roots,
        descriptors,
        frozen_list_type,
        frozen_dict_type,
    )
    return [
        _ThreadLocalModelRuntime(group, index) for index in range(len(model_roots))
    ]


def is_valid(schema_json: str, instance_json: str) -> bool:
    warnings.warn(
        "jsoncompat.is_valid(schema_json, instance_json) is deprecated; "
        "use jsoncompat.validator_for(schema_json).is_valid_json(instance_json) instead.",
        DeprecationWarning,
        stacklevel=2,
    )
    is_valid_native = _is_valid_native
    return is_valid_native(schema_json, instance_json)


__all__ = [
    "Role",
    "RoleLiteral",
    "JsonValue",
    "Generator",
    "JSONCOMPAT_MISSING",
    "JsoncompatMissingType",
    "Validator",
    "check_compat",
    "generate_value",
    "generator_for",
    "is_valid",
    "validator_for",
]

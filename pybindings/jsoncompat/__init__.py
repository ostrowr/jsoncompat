from __future__ import annotations

import importlib
import importlib.machinery
import importlib.util
import os
import sys
import warnings
from collections.abc import Mapping
from pathlib import Path
from typing import Any, Callable, Literal, NoReturn, Protocol, cast

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
CompileModelConverterFn = Callable[
    [
        list[tuple[Any, ...]],
        int,
        type[tuple[Any, ...]],
        type[Mapping[Any, Any]],
    ],
    "ModelConverter",
]
BindModelRuntimeFn = Callable[
    [type[Any], "Validator", "ModelConverter"],
    "ModelRuntime",
]


class Generator(Protocol):
    def generate_value(self, depth: int = 5) -> str: ...


class Validator(Protocol):
    def is_valid_json(self, instance_json: str) -> bool: ...
    def is_valid_value(self, instance: JsonValue) -> bool: ...
    def _is_valid_borrowed_value(self, instance: JsonValue) -> bool: ...
    def construct_value(
        self,
        instance: JsonValue,
        converter: ModelConverter,
    ) -> Any | None: ...
    def construct_kwargs(
        self,
        kwargs: dict[str, Any],
        converter: ModelConverter,
        validate: bool = True,
    ) -> Any | None: ...
    def construct_json(
        self,
        payload: str | bytes,
        converter: ModelConverter,
        validate: bool = True,
    ) -> Any | None: ...
    def model_to_value(
        self,
        instance: Any,
        converter: ModelConverter,
        validate: bool = True,
    ) -> tuple[bool, JsonValue]: ...
    def serialize_model(
        self,
        instance: Any,
        converter: ModelConverter,
        validate: bool = True,
    ) -> str | None: ...
    def parse_json(self, payload: str | bytes) -> tuple[bool, JsonValue]: ...
    def serialize_json(self, instance: JsonValue) -> str | None: ...


class ModelConverter(Protocol):
    def construct(self, value: Any, validated: bool) -> Any: ...


class ModelRuntime(Protocol):
    def deserialize(
        self,
        payload: str | bytes,
        *,
        format: Any = "json",
        skip_validation: bool = False,
    ) -> Any: ...


class NativeModule(Protocol):
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

    def compile_model_converter(
        self,
        descriptors: list[tuple[Any, ...]],
        root: int,
        frozen_list_type: type[tuple[Any, ...]],
        frozen_dict_type: type[Mapping[Any, Any]],
    ) -> ModelConverter: ...

    def bind_model_runtime(
        self,
        model_type: type[Any],
        validator: Validator,
        converter: ModelConverter,
    ) -> ModelRuntime: ...

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


def _missing_compile_model_converter(
    descriptors: list[tuple[Any, ...]],
    root: int,
    frozen_list_type: type[tuple[Any, ...]],
    frozen_dict_type: type[Mapping[Any, Any]],
) -> NoReturn:
    _ = (descriptors, root, frozen_list_type, frozen_dict_type)
    raise ModuleNotFoundError(
        "jsoncompat._native is unavailable. Install the built jsoncompat wheel "
        "before constructing generated models."
    )


def _missing_bind_model_runtime(
    model_type: type[Any],
    validator: Validator,
    converter: ModelConverter,
) -> NoReturn:
    _ = (model_type, validator, converter)
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
            for filename in ("libjsoncompat.dylib", "libjsoncompat.so", "jsoncompat.dll"):
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
    return (
        hasattr(module, "generator_for")
        and hasattr(module, "validator_for")
        and hasattr(module, "deserialize_json")
        and hasattr(module, "serialize_json")
        and hasattr(module, "compile_model_converter")
        and hasattr(module, "bind_model_runtime")
        and hasattr(validator, "is_valid_json")
        and hasattr(validator, "is_valid_value")
        and hasattr(validator, "_is_valid_borrowed_value")
        and hasattr(validator, "construct_value")
        and hasattr(validator, "construct_kwargs")
        and hasattr(validator, "construct_json")
        and hasattr(validator, "model_to_value")
        and hasattr(validator, "serialize_model")
        and hasattr(validator, "parse_json")
        and hasattr(validator, "serialize_json")
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
        _check_compat_native: CheckCompatFn = _missing_check_compat
        _generate_value_native: GenerateValueFn = _missing_generate_value
        _generator_for_native: GeneratorForFn = _missing_generator_for
        _validator_for_native: ValidatorForFn = _missing_validator_for
        _deserialize_json_native: DeserializeJsonFn = _missing_deserialize_json
        _serialize_json_native: SerializeJsonFn = _missing_serialize_json
        _compile_model_converter_native: CompileModelConverterFn = (
            _missing_compile_model_converter
        )
        _bind_model_runtime_native: BindModelRuntimeFn = _missing_bind_model_runtime
        _is_valid_native: IsValidFn = _missing_is_valid
    else:
        _check_compat_native = _repo_native.check_compat
        _generate_value_native = _repo_native.generate_value
        _generator_for_native = _repo_native.generator_for
        _validator_for_native = _repo_native.validator_for
        _deserialize_json_native = _repo_native.deserialize_json
        _serialize_json_native = _repo_native.serialize_json
        _compile_model_converter_native = _repo_native.compile_model_converter
        _bind_model_runtime_native = _repo_native.bind_model_runtime
        _is_valid_native = _repo_native.is_valid
else:
    if not _force_repo_native and not _has_reusable_schema_api(_native_module):
        _native_module = _load_repo_native()
    _check_compat_native = _native_module.check_compat
    _generate_value_native = _native_module.generate_value
    _generator_for_native = _native_module.generator_for
    _validator_for_native = _native_module.validator_for
    _deserialize_json_native = _native_module.deserialize_json
    _serialize_json_native = _native_module.serialize_json
    _compile_model_converter_native = _native_module.compile_model_converter
    _bind_model_runtime_native = _native_module.bind_model_runtime
    _is_valid_native = _native_module.is_valid


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


def generator_for(schema_json: str) -> Generator:
    generator_for_native = _generator_for_native
    return generator_for_native(schema_json)


def validator_for(schema_json: str) -> Validator:
    validator_for_native = _validator_for_native
    return validator_for_native(schema_json)


def deserialize_json_value(payload: str | bytes) -> JsonValue:
    deserialize_json_native = _deserialize_json_native
    return deserialize_json_native(payload)


def serialize_json_value(value: JsonValue) -> str:
    serialize_json_native = _serialize_json_native
    return serialize_json_native(value)


def compile_model_converter(
    descriptors: list[tuple[Any, ...]],
    root: int,
    frozen_list_type: type[tuple[Any, ...]],
    frozen_dict_type: type[Mapping[Any, Any]],
) -> ModelConverter:
    compile_native = _compile_model_converter_native
    return compile_native(descriptors, root, frozen_list_type, frozen_dict_type)


def bind_model_runtime(
    model_type: type[Any],
    validator: Validator,
    converter: ModelConverter,
) -> ModelRuntime:
    bind_native = _bind_model_runtime_native
    return bind_native(model_type, validator, converter)


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
    "Validator",
    "check_compat",
    "generate_value",
    "generator_for",
    "is_valid",
    "validator_for",
]

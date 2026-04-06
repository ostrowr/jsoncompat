from __future__ import annotations

import importlib
import importlib.machinery
import importlib.util
import sys
import warnings
from pathlib import Path
from typing import Callable, Literal, NoReturn, Protocol, cast

RoleLiteral = Literal["serializer", "deserializer", "both"]
CheckCompatFn = Callable[[str, str, RoleLiteral], bool]
GenerateValueFn = Callable[[str, int], str]
GeneratorForFn = Callable[[str], "Generator"]
ValidatorForFn = Callable[[str], "Validator"]


class Generator(Protocol):
    def generate_value(self, depth: int = 5) -> str: ...


class Validator(Protocol):
    def is_valid(self, instance_json: str) -> bool: ...


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


def _missing_validator_for(schema_json: str) -> NoReturn:
    _ = schema_json
    raise ModuleNotFoundError(
        "jsoncompat._native is unavailable. Install the built jsoncompat wheel "
        "before calling validator_for()."
    )


def _load_repo_native() -> NativeModule:
    package_dir = Path(__file__).resolve().parent
    repo_root = package_dir.parent.parent
    for build_dir in ("debug", "release"):
        for filename in ("libjsoncompat.dylib", "libjsoncompat.so", "jsoncompat.dll"):
            native_path = repo_root / "target" / build_dir / "deps" / filename
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
    return hasattr(module, "generator_for") and hasattr(module, "validator_for")


try:
    _native_module = cast(NativeModule, importlib.import_module("jsoncompat._native"))
except ModuleNotFoundError as error:
    if error.name != "jsoncompat._native":
        raise
    try:
        _repo_native = _load_repo_native()
    except ModuleNotFoundError:
        _check_compat_native: CheckCompatFn = _missing_check_compat
        _generate_value_native: GenerateValueFn = _missing_generate_value
        _generator_for_native: GeneratorForFn = _missing_generator_for
        _validator_for_native: ValidatorForFn = _missing_validator_for
    else:
        _check_compat_native = _repo_native.check_compat
        _generate_value_native = _repo_native.generate_value
        _generator_for_native = _repo_native.generator_for
        _validator_for_native = _repo_native.validator_for
else:
    if not _has_reusable_schema_api(_native_module):
        _native_module = _load_repo_native()
    _check_compat_native = _native_module.check_compat
    _generate_value_native = _native_module.generate_value
    _generator_for_native = _native_module.generator_for
    _validator_for_native = _native_module.validator_for


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


__all__ = [
    "Role",
    "RoleLiteral",
    "Generator",
    "Validator",
    "check_compat",
    "generate_value",
    "generator_for",
    "validator_for",
]

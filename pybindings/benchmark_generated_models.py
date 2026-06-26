"""Load checked-in Python models emitted by the canonical dataclass codegen."""

from __future__ import annotations

import hashlib
import importlib.util
import sys
from dataclasses import is_dataclass
from pathlib import Path
from types import ModuleType
from typing import Any


MODEL_ROOT = (
    Path(__file__).resolve().parents[1]
    / "tests"
    / "fixtures"
    / "dataclasses"
    / "benchmarks"
)


def load_generated_path(path: Path) -> ModuleType:
    path = path.resolve()
    digest = hashlib.sha256(str(path).encode()).hexdigest()[:16]
    module_name = f"_jsoncompat_generated_benchmark_{digest}"
    existing = sys.modules.get(module_name)
    if existing is not None:
        return existing

    spec = importlib.util.spec_from_file_location(module_name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"could not import generated benchmark model {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    try:
        spec.loader.exec_module(module)
    except BaseException:
        sys.modules.pop(module_name, None)
        raise
    return module


def load_generated_module(name: str) -> ModuleType:
    return load_generated_path(MODEL_ROOT / f"{name}.py")


def generated_dataclass(module: ModuleType, name: str) -> type[Any]:
    value = getattr(module, name, None)
    if not isinstance(value, type) or not is_dataclass(value):
        raise RuntimeError(
            f"generated module {module.__name__} does not export dataclass {name}"
        )
    return value

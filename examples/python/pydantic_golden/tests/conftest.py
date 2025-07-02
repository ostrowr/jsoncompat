import sys
import os
from dataclasses import dataclass
from typing import Literal, Callable, Protocol
import pytest

# Ensure the src directory is in sys.path for imports like
# 'from demo import schemas'
src_dir = os.path.join(os.path.dirname(__file__), "..", "src")
src_path = os.path.abspath(src_dir)
if src_path not in sys.path:
    sys.path.insert(0, src_path)


@dataclass
class RegistryEntry:
    stable_id: str
    mode: Literal["serializer", "deserializer", "both"]
    schema: dict


_registry: dict[str, RegistryEntry] = {}


@pytest.fixture(autouse=True)
def registry():
    yield _registry


class HasJsonSchema(Protocol):
    def model_json_schema(self) -> dict: ...


def _check_compatibility_test_patch[C: HasJsonSchema](
    stable_id: str, mode: Literal["serializer", "deserializer", "both"] = "both"
) -> Callable[[C], C]:
    def decorator_inner(cls: C) -> C:
        schema: dict = cls.model_json_schema()
        if stable_id in _registry:
            raise ValueError(
                f"Stable ID `{stable_id}` already registered. All stable IDs must be unique."
            )
        _registry[stable_id] = RegistryEntry(
            stable_id=stable_id,
            mode=mode,
            schema=schema,
        )
        return cls

    return decorator_inner


def pytest_configure() -> None:
    """Run once, as soon as pytest starts collecting tests."""
    from demo import decorator

    decorator.check_compatibility = _check_compatibility_test_patch

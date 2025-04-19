"""Typing stubs for json_schema_py

These stubs are included in the sdist / wheel so that static type checkers
(mypy, pyright, etc.) have accurate information even though the actual module
is a native extension produced by PyO3.
"""

from typing import Literal

RoleLiteral = Literal["serializer", "deserializer", "both"]

def check_compat_py(
    old_schema_json: str,
    new_schema_json: str,
    role: RoleLiteral = "both",
) -> bool: ...
def generate_value_py(
    schema_json: str,
    depth: int = 5,
) -> str: ...

__all__: list[str]

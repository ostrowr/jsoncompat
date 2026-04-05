"""Typing stubs for jsoncompat Python package"""

from typing import Final, Literal

RoleLiteral = Literal["serializer", "deserializer", "both"]

class _Role:
    SERIALIZER: Final[Literal["serializer"]]
    DESERIALIZER: Final[Literal["deserializer"]]
    BOTH: Final[Literal["both"]]

Role: _Role

def check_compat(
    old_schema_json: str, new_schema_json: str, role: RoleLiteral = "both"
) -> bool: ...
def generate_value(schema_json: str, depth: int = 5) -> str: ...

__all__: list[str]

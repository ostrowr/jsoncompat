"""Typing stubs for jsoncompat Python package"""

from typing import Final, Literal, TypeAlias

RoleLiteral = Literal["serializer", "deserializer", "both"]
JsonValue: TypeAlias = (
    None
    | bool
    | int
    | float
    | str
    | list["JsonValue"]
    | tuple["JsonValue", ...]
    | dict[str, "JsonValue"]
)

class _Role:
    SERIALIZER: Final[Literal["serializer"]]
    DESERIALIZER: Final[Literal["deserializer"]]
    BOTH: Final[Literal["both"]]

Role: _Role

class Validator:
    def is_valid(self, instance_json: str) -> bool: ...
    def is_valid_json(self, instance_json: str) -> bool: ...
    def is_valid_value(self, instance: JsonValue) -> bool: ...

def check_compat(
    old_schema_json: str, new_schema_json: str, role: RoleLiteral = "both"
) -> bool: ...
def generate_value(schema_json: str, depth: int = 5) -> str: ...
def validator_for(schema_json: str) -> Validator: ...

__all__: list[str]

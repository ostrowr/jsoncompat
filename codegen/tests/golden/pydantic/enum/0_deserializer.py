"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    1,
    2,
    3
  ]
}

Tests:
[
  {
    "data": 1,
    "description": "one of the enum is valid",
    "valid": true
  },
  {
    "data": 4,
    "description": "something else is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

class Enum0Deserializer(DeserializerRootModel):
    root: Annotated[Literal[1, 2, 3], BeforeValidator(lambda v, _allowed=[1, 2, 3]: _validate_literal(v, _allowed))]


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    false
  ]
}

Tests:
[
  {
    "data": false,
    "description": "false is valid",
    "valid": true
  },
  {
    "data": 0,
    "description": "integer zero is invalid",
    "valid": false
  },
  {
    "data": 0.0,
    "description": "float zero is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

class Enum5Serializer(SerializerRootModel):
    root: Annotated[Literal[False], BeforeValidator(lambda v, _allowed=[False]: _validate_literal(v, _allowed))]


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    [
      1
    ]
  ]
}

Tests:
[
  {
    "data": [
      true
    ],
    "description": "[true] is invalid",
    "valid": false
  },
  {
    "data": [
      1
    ],
    "description": "[1] is valid",
    "valid": true
  },
  {
    "data": [
      1.0
    ],
    "description": "[1.0] is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

class Enum12Serializer(SerializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _allowed=[[1]]: _validate_literal(v, _allowed))]


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    [
      0
    ]
  ]
}

Tests:
[
  {
    "data": [
      false
    ],
    "description": "[false] is invalid",
    "valid": false
  },
  {
    "data": [
      0
    ],
    "description": "[0] is valid",
    "valid": true
  },
  {
    "data": [
      0.0
    ],
    "description": "[0.0] is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

class Enum10Serializer(SerializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _allowed=[[0]]: _validate_literal(v, _allowed))]


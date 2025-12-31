"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    [
      false
    ]
  ]
}

Tests:
[
  {
    "data": [
      false
    ],
    "description": "[false] is valid",
    "valid": true
  },
  {
    "data": [
      0
    ],
    "description": "[0] is invalid",
    "valid": false
  },
  {
    "data": [
      0.0
    ],
    "description": "[0.0] is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

class Enum6Serializer(SerializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _allowed=[[False]]: _validate_literal(v, _allowed))]


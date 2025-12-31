"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": [
    true
  ]
}

Tests:
[
  {
    "data": [
      true
    ],
    "description": "[true] is valid",
    "valid": true
  },
  {
    "data": [
      1
    ],
    "description": "[1] is invalid",
    "valid": false
  },
  {
    "data": [
      1.0
    ],
    "description": "[1.0] is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

class Const7Serializer(SerializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _allowed=[[True]]: _validate_literal(v, _allowed))]


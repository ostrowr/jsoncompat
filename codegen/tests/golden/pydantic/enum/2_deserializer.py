"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    6,
    null
  ]
}

Tests:
[
  {
    "data": null,
    "description": "null is valid",
    "valid": true
  },
  {
    "data": 6,
    "description": "number is valid",
    "valid": true
  },
  {
    "data": "test",
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

class Enum2Deserializer(DeserializerRootModel):
    root: Annotated[Literal[6, None], BeforeValidator(lambda v, _allowed=[6, None]: _validate_literal(v, _allowed))]


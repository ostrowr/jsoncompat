"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": -2.0
}

Tests:
[
  {
    "data": -2,
    "description": "integer -2 is valid",
    "valid": true
  },
  {
    "data": 2,
    "description": "integer 2 is invalid",
    "valid": false
  },
  {
    "data": -2.0,
    "description": "float -2.0 is valid",
    "valid": true
  },
  {
    "data": 2.0,
    "description": "float 2.0 is invalid",
    "valid": false
  },
  {
    "data": -2.00001,
    "description": "float -2.00001 is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

class Const12Deserializer(DeserializerRootModel):
    root: Annotated[Literal[-2.0], BeforeValidator(lambda v, _allowed=[-2.0]: _validate_literal(v, _allowed))]


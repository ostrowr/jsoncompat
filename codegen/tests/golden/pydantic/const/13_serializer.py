"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": 9007199254740992
}

Tests:
[
  {
    "data": 9007199254740992,
    "description": "integer is valid",
    "valid": true
  },
  {
    "data": 9007199254740991,
    "description": "integer minus one is invalid",
    "valid": false
  },
  {
    "data": 9007199254740992.0,
    "description": "float is valid",
    "valid": true
  },
  {
    "data": 9007199254740990.0,
    "description": "float minus one is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

class Const13Serializer(SerializerRootModel):
    root: Annotated[Literal[9007199254740992], BeforeValidator(lambda v, _allowed=[9007199254740992]: _validate_literal(v, _allowed))]


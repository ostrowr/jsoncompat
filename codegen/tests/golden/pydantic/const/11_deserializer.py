"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": 1
}

Tests:
[
  {
    "data": true,
    "description": "true is invalid",
    "valid": false
  },
  {
    "data": 1,
    "description": "integer one is valid",
    "valid": true
  },
  {
    "data": 1.0,
    "description": "float one is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

class Const11Deserializer(DeserializerRootModel):
    root: Annotated[Literal[1], BeforeValidator(lambda v, _allowed=[1]: _validate_literal(v, _allowed))]


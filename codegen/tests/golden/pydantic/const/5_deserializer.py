"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": true
}

Tests:
[
  {
    "data": true,
    "description": "true is valid",
    "valid": true
  },
  {
    "data": 1,
    "description": "integer one is invalid",
    "valid": false
  },
  {
    "data": 1.0,
    "description": "float one is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

class Const5Deserializer(DeserializerRootModel):
    root: Annotated[Literal[True], BeforeValidator(lambda v, _allowed=[True]: _validate_literal(v, _allowed))]


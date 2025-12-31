"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": 2
}

Tests:
[
  {
    "data": 2,
    "description": "same value is valid",
    "valid": true
  },
  {
    "data": 5,
    "description": "another value is invalid",
    "valid": false
  },
  {
    "data": "a",
    "description": "another type is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

class Const0Deserializer(DeserializerRootModel):
    root: Annotated[Literal[2], BeforeValidator(lambda v, _allowed=[2]: _validate_literal(v, _allowed))]


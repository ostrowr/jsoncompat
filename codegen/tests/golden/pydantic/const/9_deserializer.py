"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": {
    "a": true
  }
}

Tests:
[
  {
    "data": {
      "a": true
    },
    "description": "{\"a\": true} is valid",
    "valid": true
  },
  {
    "data": {
      "a": 1
    },
    "description": "{\"a\": 1} is invalid",
    "valid": false
  },
  {
    "data": {
      "a": 1.0
    },
    "description": "{\"a\": 1.0} is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

class Const9Deserializer(DeserializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _allowed=[{"a": True}]: _validate_literal(v, _allowed))]


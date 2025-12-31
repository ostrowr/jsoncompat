"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": [
    {
      "foo": "bar"
    }
  ]
}

Tests:
[
  {
    "data": [
      {
        "foo": "bar"
      }
    ],
    "description": "same array is valid",
    "valid": true
  },
  {
    "data": [
      2
    ],
    "description": "another array item is invalid",
    "valid": false
  },
  {
    "data": [
      1,
      2,
      3
    ],
    "description": "array with additional items is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

class Const2Deserializer(DeserializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _allowed=[[{"foo": "bar"}]]: _validate_literal(v, _allowed))]


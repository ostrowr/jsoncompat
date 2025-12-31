"""
Schema:
{
  "minLength": 2
}

Tests:
[
  {
    "data": "foo",
    "description": "a 3-character string is valid",
    "valid": true
  },
  {
    "data": "a",
    "description": "a 1-character string is not valid",
    "valid": false
  },
  {
    "data": 5,
    "description": "a non-string is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Noschema0Deserializer(DeserializerRootModel):
    root: Annotated[str, Field(min_length=2)]


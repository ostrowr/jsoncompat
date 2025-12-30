"""
Schema:
{
  "$defs": {
    "a": {
      "type": "integer"
    },
    "b": {
      "$ref": "#/$defs/a"
    },
    "c": {
      "$ref": "#/$defs/b"
    }
  },
  "$ref": "#/$defs/c",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": 5,
    "description": "nested ref valid",
    "valid": true
  },
  {
    "data": "a",
    "description": "nested ref invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ref4Serializer(SerializerRootModel):
    root: int


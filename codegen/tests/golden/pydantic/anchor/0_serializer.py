"""
Schema:
{
  "$defs": {
    "A": {
      "$anchor": "foo",
      "type": "integer"
    }
  },
  "$ref": "#foo",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": 1,
    "description": "match",
    "valid": true
  },
  {
    "data": "a",
    "description": "mismatch",
    "valid": false
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Anchor0Serializer(SerializerRootModel):
    root: int


"""
Schema:
{
  "$defs": {
    "foo": {
      "type": "number"
    }
  },
  "$id": "file:///folder/file.json",
  "$ref": "#/$defs/foo",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": 1,
    "description": "number is valid",
    "valid": true
  },
  {
    "data": "a",
    "description": "non-number is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ref32Serializer(SerializerRootModel):
    root: float


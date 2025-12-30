"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": [
    "string"
  ]
}

Tests:
[
  {
    "data": "foo",
    "description": "string is valid",
    "valid": true
  },
  {
    "data": 123,
    "description": "number is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Type8Deserializer(DeserializerRootModel):
    root: str


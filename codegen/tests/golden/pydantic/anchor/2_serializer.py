"""
Schema:
{
  "$defs": {
    "A": {
      "$defs": {
        "B": {
          "$anchor": "foo",
          "type": "integer"
        }
      },
      "$id": "nested.json"
    }
  },
  "$id": "http://localhost:1234/draft2020-12/root",
  "$ref": "http://localhost:1234/draft2020-12/nested.json#foo",
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

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Anchor2Serializer(SerializerRootModel):
    root: Any


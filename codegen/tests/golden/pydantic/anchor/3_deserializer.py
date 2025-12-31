"""
Schema:
{
  "$defs": {
    "A": {
      "$id": "child1",
      "allOf": [
        {
          "$anchor": "my_anchor",
          "$id": "child2",
          "type": "number"
        },
        {
          "$anchor": "my_anchor",
          "type": "string"
        }
      ]
    }
  },
  "$id": "http://localhost:1234/draft2020-12/foobar",
  "$ref": "child1#my_anchor",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": "a",
    "description": "$ref resolves to /$defs/A/allOf/1",
    "valid": true
  },
  {
    "data": 1,
    "description": "$ref does not resolve to /$defs/A/allOf/0",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Anchor3Deserializer(DeserializerRootModel):
    root: Any


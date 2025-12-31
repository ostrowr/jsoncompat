"""
Schema:
{
  "$defs": {
    "foo": {
      "$dynamicAnchor": "items",
      "type": "string"
    },
    "list": {
      "$defs": {
        "items": {
          "$comment": "This is only needed to satisfy the bookending requirement",
          "$dynamicAnchor": "items",
          "type": "number"
        }
      },
      "$id": "list",
      "items": {
        "$dynamicRef": "#/$defs/items"
      },
      "type": "array"
    }
  },
  "$id": "https://test.json-schema.org/dynamicRef-without-anchor/root",
  "$ref": "list",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": [
      "foo",
      "bar"
    ],
    "description": "An array of strings is invalid",
    "valid": false
  },
  {
    "data": [
      24,
      42
    ],
    "description": "An array of numbers is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Dynamicref4Serializer(SerializerRootModel):
    root: list[Any]


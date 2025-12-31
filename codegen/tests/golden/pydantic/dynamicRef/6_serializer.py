"""
Schema:
{
  "$defs": {
    "foo": {
      "$anchor": "items",
      "type": "string"
    },
    "list": {
      "$defs": {
        "items": {
          "$comment": "This is only needed to satisfy the bookending requirement",
          "$dynamicAnchor": "items"
        }
      },
      "$id": "list",
      "items": {
        "$dynamicRef": "#items"
      },
      "type": "array"
    }
  },
  "$id": "https://test.json-schema.org/dynamic-resolution-ignores-anchors/root",
  "$ref": "list",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": [
      "foo",
      42
    ],
    "description": "Any array is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Dynamicref6Serializer(SerializerRootModel):
    root: list[Any]


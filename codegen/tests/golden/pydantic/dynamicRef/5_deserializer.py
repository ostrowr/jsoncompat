"""
Schema:
{
  "$defs": {
    "foo": {
      "$dynamicAnchor": "items",
      "type": "string"
    },
    "intermediate-scope": {
      "$id": "intermediate-scope",
      "$ref": "list"
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
  "$id": "https://test.json-schema.org/dynamic-resolution-with-intermediate-scopes/root",
  "$ref": "intermediate-scope",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": [
      "foo",
      "bar"
    ],
    "description": "An array of strings is valid",
    "valid": true
  },
  {
    "data": [
      "foo",
      42
    ],
    "description": "An array containing non-strings is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Dynamicref5Deserializer(DeserializerRootModel):
    root: list[Any]


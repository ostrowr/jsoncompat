"""
Schema:
{
  "$defs": {
    "foo": {
      "$dynamicAnchor": "items",
      "type": "string"
    }
  },
  "$id": "https://test.json-schema.org/ref-dynamicAnchor-same-schema/root",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "$ref": "#items"
  },
  "type": "array"
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

class Dynamicref2Deserializer(DeserializerRootModel):
    root: list[Any]


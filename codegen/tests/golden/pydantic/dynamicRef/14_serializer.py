"""
Schema:
{
  "$defs": {
    "elements": {
      "$dynamicAnchor": "elements",
      "additionalProperties": false,
      "properties": {
        "a": true
      },
      "required": [
        "a"
      ]
    }
  },
  "$id": "http://localhost:1234/draft2020-12/strict-extendible.json",
  "$ref": "extendible-dynamic-ref.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": {
      "a": true
    },
    "description": "incorrect parent schema",
    "valid": false
  },
  {
    "data": {
      "elements": [
        {
          "b": 1
        }
      ]
    },
    "description": "incorrect extended schema",
    "valid": false
  },
  {
    "data": {
      "elements": [
        {
          "a": 1
        }
      ]
    },
    "description": "correct extended schema",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Dynamicref14Serializer(SerializerRootModel):
    root: Any


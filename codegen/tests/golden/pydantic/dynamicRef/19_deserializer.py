"""
Schema:
{
  "$defs": {
    "bar": {
      "$defs": {
        "content": {
          "$dynamicAnchor": "content",
          "type": "string"
        },
        "item": {
          "$defs": {
            "defaultContent": {
              "$dynamicAnchor": "content",
              "type": "integer"
            }
          },
          "$id": "item",
          "properties": {
            "content": {
              "$dynamicRef": "#content"
            }
          },
          "type": "object"
        }
      },
      "$id": "bar",
      "items": {
        "$ref": "item"
      },
      "type": "array"
    }
  },
  "$id": "https://test.json-schema.org/dynamic-ref-skips-intermediate-resource/main",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar-item": {
      "$ref": "item"
    }
  },
  "type": "object"
}

Tests:
[
  {
    "data": {
      "bar-item": {
        "content": 42
      }
    },
    "description": "integer property passes",
    "valid": true
  },
  {
    "data": {
      "bar-item": {
        "content": "value"
      }
    },
    "description": "string property fails",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Dynamicref19Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    bar_item: Annotated[Any | None, Field(alias="bar-item", default=None)]


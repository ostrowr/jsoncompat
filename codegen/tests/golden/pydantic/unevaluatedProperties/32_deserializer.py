"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "x": {
      "$ref": "#"
    }
  },
  "type": "object",
  "unevaluatedProperties": false
}

Tests:
[
  {
    "data": {},
    "description": "Empty is valid",
    "valid": true
  },
  {
    "data": {
      "x": {}
    },
    "description": "Single is valid",
    "valid": true
  },
  {
    "data": {
      "x": {},
      "y": {}
    },
    "description": "Unevaluated on 1st level is invalid",
    "valid": false
  },
  {
    "data": {
      "x": {
        "x": {}
      }
    },
    "description": "Nested is valid",
    "valid": true
  },
  {
    "data": {
      "x": {
        "x": {},
        "y": {}
      }
    },
    "description": "Unevaluated on 2nd level is invalid",
    "valid": false
  },
  {
    "data": {
      "x": {
        "x": {
          "x": {}
        }
      }
    },
    "description": "Deep nested is valid",
    "valid": true
  },
  {
    "data": {
      "x": {
        "x": {
          "x": {},
          "y": {}
        }
      }
    },
    "description": "Unevaluated on 3rd level is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Unevaluatedproperties32Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    x: Annotated[Any | None, Field(default=None)]


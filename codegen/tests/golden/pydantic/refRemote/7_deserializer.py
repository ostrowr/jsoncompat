"""
Schema:
{
  "$id": "http://localhost:1234/draft2020-12/object",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "name": {
      "$ref": "name-defs.json#/$defs/orNull"
    }
  },
  "type": "object"
}

Tests:
[
  {
    "data": {
      "name": "foo"
    },
    "description": "string is valid",
    "valid": true
  },
  {
    "data": {
      "name": null
    },
    "description": "null is valid",
    "valid": true
  },
  {
    "data": {
      "name": {
        "name": null
      }
    },
    "description": "object is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Refremote7Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    name: Annotated[Any | None, Field(default=None)]


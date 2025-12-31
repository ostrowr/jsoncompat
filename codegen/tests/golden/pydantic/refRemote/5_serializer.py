"""
Schema:
{
  "$defs": {
    "baz": {
      "$id": "baseUriChangeFolder/",
      "items": {
        "$ref": "folderInteger.json"
      },
      "type": "array"
    }
  },
  "$id": "http://localhost:1234/draft2020-12/scope_change_defs1.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "list": {
      "$ref": "baseUriChangeFolder/"
    }
  },
  "type": "object"
}

Tests:
[
  {
    "data": {
      "list": [
        1
      ]
    },
    "description": "number is valid",
    "valid": true
  },
  {
    "data": {
      "list": [
        "a"
      ]
    },
    "description": "string is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Refremote5Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    list: Annotated[list[Any] | None, Field(default=None)]


"""
Schema:
{
  "$defs": {
    "baz": {
      "$defs": {
        "bar": {
          "items": {
            "$ref": "folderInteger.json"
          },
          "type": "array"
        }
      },
      "$id": "baseUriChangeFolderInSubschema/"
    }
  },
  "$id": "http://localhost:1234/draft2020-12/scope_change_defs2.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "list": {
      "$ref": "baseUriChangeFolderInSubschema/#/$defs/bar"
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

class Refremote6Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    list: Annotated[Any | None, Field(default=None)]


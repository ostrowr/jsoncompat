"""
Schema:
{
  "$id": "http://localhost:1234/draft2020-12/some-id",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "name": {
      "$ref": "nested/foo-ref-string.json"
    }
  }
}

Tests:
[
  {
    "data": {
      "name": {
        "foo": 1
      }
    },
    "description": "number is invalid",
    "valid": false
  },
  {
    "data": {
      "name": {
        "foo": "a"
      }
    },
    "description": "string is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Refremote10Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    name: Annotated[Any | None, Field(default=None)]


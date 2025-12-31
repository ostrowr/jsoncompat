"""
Schema:
{
  "$defs": {
    "int": {
      "type": "integer"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "properties": {
        "foo": {
          "$ref": "#/$defs/int"
        }
      }
    },
    {
      "additionalProperties": {
        "$ref": "#/$defs/int"
      }
    }
  ]
}

Tests:
[
  {
    "data": {
      "foo": 1
    },
    "description": "passing case",
    "valid": true
  },
  {
    "data": {
      "foo": "a string"
    },
    "description": "failing case",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Infiniteloopdetection0Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    __pydantic_extra__: dict[str, int]
    foo: Annotated[int | None, Field(default=None)]


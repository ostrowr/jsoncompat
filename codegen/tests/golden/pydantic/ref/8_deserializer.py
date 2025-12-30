"""
Schema:
{
  "$defs": {
    "is-string": {
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "$ref": {
      "$ref": "#/$defs/is-string"
    }
  }
}

Tests:
[
  {
    "data": {
      "$ref": "a"
    },
    "description": "property named $ref valid",
    "valid": true
  },
  {
    "data": {
      "$ref": 2
    },
    "description": "property named $ref invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Ref8Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    ref: Annotated[str | None, Field(alias="$ref", default=None)]


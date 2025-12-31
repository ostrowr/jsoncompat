"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "type": "boolean"
  }
}

Tests:
[
  {
    "data": {
      "foo": true
    },
    "description": "an additional valid property is valid",
    "valid": true
  },
  {
    "data": {
      "foo": 1
    },
    "description": "an additional invalid property is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Additionalproperties3Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    __pydantic_extra__: dict[str, bool]


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "type": "null"
  }
}

Tests:
[
  {
    "data": {
      "foo": null
    },
    "description": "allows null values",
    "valid": true
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Additionalproperties6Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    __pydantic_extra__: dict[str, None]


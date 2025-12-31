"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "type": "boolean"
  },
  "allOf": [
    {
      "properties": {
        "foo": {}
      }
    }
  ]
}

Tests:
[
  {
    "data": {
      "bar": true,
      "foo": 1
    },
    "description": "properties defined in allOf are not examined",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Additionalproperties5Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    __pydantic_extra__: dict[str, bool]
    foo: Annotated[Any | None, Field(default=None)]


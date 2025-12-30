"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "type": "boolean"
  },
  "properties": {
    "bar": {},
    "foo": {}
  }
}

Tests:
[
  {
    "data": {
      "foo": 1
    },
    "description": "no additional properties is valid",
    "valid": true
  },
  {
    "data": {
      "bar": 2,
      "foo": 1,
      "quux": true
    },
    "description": "an additional valid property is valid",
    "valid": true
  },
  {
    "data": {
      "bar": 2,
      "foo": 1,
      "quux": 12
    },
    "description": "an additional invalid property is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Additionalproperties2Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    __pydantic_extra__: dict[str, bool]
    bar: Annotated[Any | None, Field(default=None)]
    foo: Annotated[Any | None, Field(default=None)]


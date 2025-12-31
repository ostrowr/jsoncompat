"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "patternProperties": {
    "^v": {}
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
      "quux": "boom"
    },
    "description": "an additional property is invalid",
    "valid": false
  },
  {
    "data": [
      1,
      2,
      3
    ],
    "description": "ignores arrays",
    "valid": true
  },
  {
    "data": "foobarbaz",
    "description": "ignores strings",
    "valid": true
  },
  {
    "data": 12,
    "description": "ignores other non-objects",
    "valid": true
  },
  {
    "data": {
      "foo": 1,
      "vroom": 2
    },
    "description": "patternProperties are not additional properties",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Additionalproperties0Serializer(SerializerBase):
    model_config = ConfigDict(extra="forbid")
    bar: Annotated[Any | None, Field(default=None)]
    foo: Annotated[Any | None, Field(default=None)]


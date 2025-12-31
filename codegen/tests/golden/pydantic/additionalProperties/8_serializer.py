"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "dependentSchemas": {
    "foo": {},
    "foo2": {
      "properties": {
        "bar": {}
      }
    }
  },
  "properties": {
    "foo2": {}
  }
}

Tests:
[
  {
    "data": {
      "foo": ""
    },
    "description": "additionalProperties doesn't consider dependentSchemas",
    "valid": false
  },
  {
    "data": {
      "bar": ""
    },
    "description": "additionalProperties can't see bar",
    "valid": false
  },
  {
    "data": {
      "bar": "",
      "foo2": ""
    },
    "description": "additionalProperties can't see bar even when foo2 is present",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Additionalproperties8Serializer(SerializerBase):
    model_config = ConfigDict(extra="forbid")
    foo2: Annotated[Any | None, Field(default=None)]


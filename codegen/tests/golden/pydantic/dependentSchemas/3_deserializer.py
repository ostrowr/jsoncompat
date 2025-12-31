"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "dependentSchemas": {
    "foo": {
      "additionalProperties": false,
      "properties": {
        "bar": {}
      }
    }
  },
  "properties": {
    "foo": {}
  }
}

Tests:
[
  {
    "data": {
      "foo": 1
    },
    "description": "matches root",
    "valid": false
  },
  {
    "data": {
      "bar": 1
    },
    "description": "matches dependency",
    "valid": true
  },
  {
    "data": {
      "bar": 2,
      "foo": 1
    },
    "description": "matches both",
    "valid": false
  },
  {
    "data": {
      "baz": 1
    },
    "description": "no dependency",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Dependentschemas3Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    foo: Annotated[Any | None, Field(default=None)]


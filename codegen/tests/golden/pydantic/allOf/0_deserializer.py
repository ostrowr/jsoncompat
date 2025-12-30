"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "properties": {
        "bar": {
          "type": "integer"
        }
      },
      "required": [
        "bar"
      ]
    },
    {
      "properties": {
        "foo": {
          "type": "string"
        }
      },
      "required": [
        "foo"
      ]
    }
  ]
}

Tests:
[
  {
    "data": {
      "bar": 2,
      "foo": "baz"
    },
    "description": "allOf",
    "valid": true
  },
  {
    "data": {
      "foo": "baz"
    },
    "description": "mismatch second",
    "valid": false
  },
  {
    "data": {
      "bar": 2
    },
    "description": "mismatch first",
    "valid": false
  },
  {
    "data": {
      "bar": "quux",
      "foo": "baz"
    },
    "description": "wrong type",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Allof0Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Annotated[int, Field()]
    foo: Annotated[str, Field()]


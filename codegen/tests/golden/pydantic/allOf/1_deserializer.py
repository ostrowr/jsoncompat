"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "properties": {
        "foo": {
          "type": "string"
        }
      },
      "required": [
        "foo"
      ]
    },
    {
      "properties": {
        "baz": {
          "type": "null"
        }
      },
      "required": [
        "baz"
      ]
    }
  ],
  "properties": {
    "bar": {
      "type": "integer"
    }
  },
  "required": [
    "bar"
  ]
}

Tests:
[
  {
    "data": {
      "bar": 2,
      "baz": null,
      "foo": "quux"
    },
    "description": "valid",
    "valid": true
  },
  {
    "data": {
      "baz": null,
      "foo": "quux"
    },
    "description": "mismatch base schema",
    "valid": false
  },
  {
    "data": {
      "bar": 2,
      "baz": null
    },
    "description": "mismatch first allOf",
    "valid": false
  },
  {
    "data": {
      "bar": 2,
      "foo": "quux"
    },
    "description": "mismatch second allOf",
    "valid": false
  },
  {
    "data": {
      "bar": 2
    },
    "description": "mismatch both",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Allof1Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Annotated[int, Field()]
    baz: Annotated[None, Field()]
    foo: Annotated[str, Field()]


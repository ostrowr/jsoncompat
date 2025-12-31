"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
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
      "bar": 2
    },
    "description": "first oneOf valid (complex)",
    "valid": true
  },
  {
    "data": {
      "foo": "baz"
    },
    "description": "second oneOf valid (complex)",
    "valid": true
  },
  {
    "data": {
      "bar": 2,
      "foo": "baz"
    },
    "description": "both oneOf valid (complex)",
    "valid": false
  },
  {
    "data": {
      "bar": "quux",
      "foo": 2
    },
    "description": "neither oneOf valid (complex)",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class ModelSerializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Annotated[int, Field()]

class Model2Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    foo: Annotated[str, Field()]

class Oneof6Serializer(SerializerRootModel):
    root: ModelSerializer | Model2Serializer


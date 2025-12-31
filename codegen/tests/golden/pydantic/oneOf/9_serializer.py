"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "properties": {
        "bar": true,
        "baz": true
      },
      "required": [
        "bar"
      ]
    },
    {
      "properties": {
        "foo": true
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
      "bar": 8
    },
    "description": "first oneOf valid",
    "valid": true
  },
  {
    "data": {
      "foo": "foo"
    },
    "description": "second oneOf valid",
    "valid": true
  },
  {
    "data": {
      "bar": 8,
      "foo": "foo"
    },
    "description": "both oneOf valid",
    "valid": false
  },
  {
    "data": {
      "baz": "quux"
    },
    "description": "neither oneOf valid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class ModelSerializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Annotated[Any, Field()]
    baz: Annotated[Any | None, Field(default=None)]

class Model2Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    foo: Annotated[Any, Field()]

class Oneof9Serializer(SerializerRootModel):
    root: ModelSerializer | Model2Serializer


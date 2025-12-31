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
from pydantic_core import core_schema

class ModelSerializer(SerializerBase):

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    bar: Any
    baz: Annotated[Any | None, Field(default=None)]

class Model2Serializer(SerializerBase):

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    foo: Any

class Oneof9Serializer(SerializerRootModel):
    root: ModelSerializer | Model2Serializer


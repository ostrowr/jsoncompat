"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
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
    "description": "first anyOf valid (complex)",
    "valid": true
  },
  {
    "data": {
      "foo": "baz"
    },
    "description": "second anyOf valid (complex)",
    "valid": true
  },
  {
    "data": {
      "bar": 2,
      "foo": "baz"
    },
    "description": "both anyOf valid (complex)",
    "valid": true
  },
  {
    "data": {
      "bar": "quux",
      "foo": 2
    },
    "description": "neither anyOf valid (complex)",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field
from pydantic_core import core_schema

class ModelDeserializer(DeserializerBase):

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    bar: Annotated[int, Field()]

class Model2Deserializer(DeserializerBase):

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    foo: Annotated[str, Field()]

class Anyof5Deserializer(DeserializerRootModel):
    root: ModelDeserializer | Model2Deserializer


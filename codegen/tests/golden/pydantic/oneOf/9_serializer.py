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

from typing import Annotated, Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator
from pydantic_core import core_schema

_JSON_SCHEMA = r"""
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
"""

_VALIDATE_FORMATS = False

class ModelSerializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "__root__": {
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
  },
  "$ref": "#/$defs/__root__/oneOf/0",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    bar: Any
    baz: Annotated[Any | None, Field(default=None)]

class Model2Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "__root__": {
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
  },
  "$ref": "#/$defs/__root__/oneOf/1",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    foo: Any

class Oneof9Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: ModelSerializer | Model2Serializer


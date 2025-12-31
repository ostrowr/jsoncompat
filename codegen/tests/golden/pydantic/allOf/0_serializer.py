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

from typing import ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator
from pydantic_core import core_schema

_JSON_SCHEMA = r"""
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
"""

_VALIDATE_FORMATS = False

class Allof0Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    bar: int | float
    foo: str


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

from typing import Annotated, Any, ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator
from pydantic_core import core_schema

_JSON_SCHEMA = r"""
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
"""

_VALIDATE_FORMATS = False

class Dependentschemas3Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    foo: Annotated[Any | None, Field(default=None)]


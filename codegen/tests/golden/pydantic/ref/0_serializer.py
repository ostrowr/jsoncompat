"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "foo": {
      "$ref": "#"
    }
  }
}

Tests:
[
  {
    "data": {
      "foo": false
    },
    "description": "match",
    "valid": true
  },
  {
    "data": {
      "foo": {
        "foo": false
      }
    },
    "description": "recursive match",
    "valid": true
  },
  {
    "data": {
      "bar": false
    },
    "description": "mismatch",
    "valid": false
  },
  {
    "data": {
      "foo": {
        "bar": false
      }
    },
    "description": "recursive mismatch",
    "valid": false
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
  "additionalProperties": false,
  "properties": {
    "foo": {
      "$ref": "#"
    }
  }
}
"""

_VALIDATE_FORMATS = False

class Ref0Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    foo: Annotated[Any | None, Field(default=None)]


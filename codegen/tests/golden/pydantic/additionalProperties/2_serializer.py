"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "type": "boolean"
  },
  "properties": {
    "bar": {},
    "foo": {}
  }
}

Tests:
[
  {
    "data": {
      "foo": 1
    },
    "description": "no additional properties is valid",
    "valid": true
  },
  {
    "data": {
      "bar": 2,
      "foo": 1,
      "quux": true
    },
    "description": "an additional valid property is valid",
    "valid": true
  },
  {
    "data": {
      "bar": 2,
      "foo": 1,
      "quux": 12
    },
    "description": "an additional invalid property is invalid",
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
  "additionalProperties": {
    "type": "boolean"
  },
  "properties": {
    "bar": {},
    "foo": {}
  }
}
"""

_VALIDATE_FORMATS = False

class Additionalproperties2Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    bar: Annotated[Any | None, Field(default=None)]
    foo: Annotated[Any | None, Field(default=None)]


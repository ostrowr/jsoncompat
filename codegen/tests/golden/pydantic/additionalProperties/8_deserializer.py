"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "dependentSchemas": {
    "foo": {},
    "foo2": {
      "properties": {
        "bar": {}
      }
    }
  },
  "properties": {
    "foo2": {}
  }
}

Tests:
[
  {
    "data": {
      "foo": ""
    },
    "description": "additionalProperties doesn't consider dependentSchemas",
    "valid": false
  },
  {
    "data": {
      "bar": ""
    },
    "description": "additionalProperties can't see bar",
    "valid": false
  },
  {
    "data": {
      "bar": "",
      "foo2": ""
    },
    "description": "additionalProperties can't see bar even when foo2 is present",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator
from pydantic_core import core_schema

class Additionalproperties8Deserializer(DeserializerBase):

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    foo2: Annotated[Any | None, Field(default=None)]
    _pattern_properties: ClassVar[list] = []

    @model_validator(mode="before")
    @classmethod
    def _validate_additional(cls, value):
        if not isinstance(value, dict):
            return value
        _allowed = {"foo2"}
        for _key, _val in value.items():
            if _key in _allowed:
                continue
            if cls._pattern_properties and any(p.match(_key) for p in cls._pattern_properties):
                continue
            raise ValueError("additional property not allowed")
        return value


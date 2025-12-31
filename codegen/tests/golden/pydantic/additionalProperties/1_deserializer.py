"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "patternProperties": {
    "^á": {}
  }
}

Tests:
[
  {
    "data": {
      "ármányos": 2
    },
    "description": "matching the pattern is valid",
    "valid": true
  },
  {
    "data": {
      "élmény": 2
    },
    "description": "not matching the pattern is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator
from pydantic_core import core_schema
from re import compile as re_compile

class Additionalproperties1Deserializer(DeserializerBase):

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    _pattern_properties: ClassVar[list] = [re_compile(r"^á")]

    @model_validator(mode="before")
    @classmethod
    def _validate_additional(cls, value):
        if not isinstance(value, dict):
            return value
        _allowed = set()
        for _key, _val in value.items():
            if _key in _allowed:
                continue
            if cls._pattern_properties and any(p.match(_key) for p in cls._pattern_properties):
                continue
            raise ValueError("additional property not allowed")
        return value


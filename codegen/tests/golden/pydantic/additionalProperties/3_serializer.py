"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "type": "boolean"
  }
}

Tests:
[
  {
    "data": {
      "foo": true
    },
    "description": "an additional valid property is valid",
    "valid": true
  },
  {
    "data": {
      "foo": 1
    },
    "description": "an additional invalid property is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, TypeAdapter, model_validator
from pydantic_core import core_schema

class Additionalproperties3Serializer(SerializerBase):

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")
    __pydantic_extra__: dict[str, bool]
    _pattern_properties: ClassVar[list] = []
    _additional_adapter: ClassVar[TypeAdapter] = TypeAdapter(bool, config=ConfigDict(strict=True))

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
            try:
                cls._additional_adapter.validate_python(_val)
            except Exception as exc:
                raise ValueError("additional property does not match schema") from exc
        return value


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 1
}

Tests:
[
  {
    "data": {
      "bar": 2,
      "foo": 1
    },
    "description": "longer is valid",
    "valid": true
  },
  {
    "data": {},
    "description": "too short is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator
from pydantic_core import core_schema

class Minproperties1Deserializer(DeserializerBase):

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        model_schema = handler(source)
        non_object_schema = core_schema.no_info_plain_validator_function(lambda v: v)
        return core_schema.tagged_union_schema({True: model_schema, False: non_object_schema}, discriminator=lambda v: isinstance(v, dict))
    model_config = ConfigDict(extra="allow")

    @model_validator(mode="after")
    def _check_properties(self):
        keys = set(self.model_fields_set)
        extra = getattr(self, "__pydantic_extra__", None)
        if extra:
            keys.update(extra.keys())
        if len(keys) < 1:
            raise ValueError("expected at least 1 properties")
        return self


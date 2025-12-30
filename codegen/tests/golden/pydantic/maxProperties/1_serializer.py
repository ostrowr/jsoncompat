"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "maxProperties": 2
}

Tests:
[
  {
    "data": {
      "foo": 1
    },
    "description": "shorter is valid",
    "valid": true
  },
  {
    "data": {
      "bar": 2,
      "baz": 3,
      "foo": 1
    },
    "description": "too long is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator

class Maxproperties1Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")

    @model_validator(mode="after")
    def _check_properties(self):
        keys = set(self.model_fields_set)
        extra = getattr(self, "__pydantic_extra__", None)
        if extra:
            keys.update(extra.keys())
        if len(keys) > 2:
            raise ValueError("expected at most 2 properties")
        return self


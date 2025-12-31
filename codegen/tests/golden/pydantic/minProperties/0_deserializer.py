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
    "data": {
      "foo": 1
    },
    "description": "exact length is valid",
    "valid": true
  },
  {
    "data": {},
    "description": "too short is invalid",
    "valid": false
  },
  {
    "data": [],
    "description": "ignores arrays",
    "valid": true
  },
  {
    "data": "",
    "description": "ignores strings",
    "valid": true
  },
  {
    "data": 12,
    "description": "ignores other non-objects",
    "valid": true
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator

class Minproperties0Deserializer(DeserializerBase):
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


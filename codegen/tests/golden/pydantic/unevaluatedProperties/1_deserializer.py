"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "unevaluatedProperties": {
    "minLength": 3,
    "type": "string"
  }
}

Tests:
[
  {
    "data": {},
    "description": "with no unevaluated properties",
    "valid": true
  },
  {
    "data": {
      "foo": "foo"
    },
    "description": "with valid unevaluated properties",
    "valid": true
  },
  {
    "data": {
      "foo": "fo"
    },
    "description": "with invalid unevaluated properties",
    "valid": false
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Unevaluatedproperties1Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")


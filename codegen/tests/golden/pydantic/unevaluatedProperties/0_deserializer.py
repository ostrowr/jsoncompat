"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "unevaluatedProperties": true
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
    "description": "with unevaluated properties",
    "valid": true
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Unevaluatedproperties0Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")


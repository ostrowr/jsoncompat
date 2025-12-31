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

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Additionalproperties1Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="forbid")


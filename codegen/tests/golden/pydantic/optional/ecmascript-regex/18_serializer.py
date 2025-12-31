"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "patternProperties": {
    "^\\d+$": true
  },
  "type": "object"
}

Tests:
[
  {
    "data": {
      "42": "life, the universe, and everything"
    },
    "description": "ascii digits",
    "valid": true
  },
  {
    "data": {
      "-%#": "spending the year dead for tax reasons"
    },
    "description": "ascii non-digits",
    "valid": false
  },
  {
    "data": {
      "৪২": "khajit has wares if you have coin"
    },
    "description": "non-ascii digits (BENGALI DIGIT FOUR, BENGALI DIGIT TWO)",
    "valid": false
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Ecmascriptregex18Serializer(SerializerBase):
    model_config = ConfigDict(extra="forbid")


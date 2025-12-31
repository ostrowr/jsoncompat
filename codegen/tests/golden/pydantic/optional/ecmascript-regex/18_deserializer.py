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

from typing import ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator
from re import compile as re_compile

class Ecmascriptregex18Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    _pattern_properties: ClassVar[list] = [re_compile(r"^\\d+$")]

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


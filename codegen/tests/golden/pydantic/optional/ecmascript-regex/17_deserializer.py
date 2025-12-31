"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "patternProperties": {
    "[a-z]cole": true
  },
  "type": "object"
}

Tests:
[
  {
    "data": {
      "l'école": "pas de vraie vie"
    },
    "description": "literal unicode character in json string",
    "valid": false
  },
  {
    "data": {
      "l'école": "pas de vraie vie"
    },
    "description": "unicode character in hex format in string",
    "valid": false
  },
  {
    "data": {
      "l'ecole": "pas de vraie vie"
    },
    "description": "ascii characters match",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator
from re import compile as re_compile

class Ecmascriptregex17Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    _pattern_properties: ClassVar[list] = [re_compile(r"[a-z]cole")]

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


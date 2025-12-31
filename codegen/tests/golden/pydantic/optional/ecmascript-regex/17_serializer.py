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

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Ecmascriptregex17Serializer(SerializerBase):
    model_config = ConfigDict(extra="forbid")


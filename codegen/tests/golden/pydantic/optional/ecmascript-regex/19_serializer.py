"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "patternProperties": {
    "^\\p{digit}+$": true
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
    "valid": true
  }
]
"""

from typing import ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "patternProperties": {
    "^\\p{digit}+$": true
  },
  "type": "object"
}
"""

_VALIDATE_FORMATS = False

class Ecmascriptregex19Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    model_config = ConfigDict(extra="allow")


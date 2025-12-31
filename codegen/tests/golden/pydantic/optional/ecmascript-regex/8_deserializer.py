"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\s$",
  "type": "string"
}

Tests:
[
  {
    "data": " ",
    "description": "ASCII space matches",
    "valid": true
  },
  {
    "data": "\t",
    "description": "Character tabulation matches",
    "valid": true
  },
  {
    "data": "\u000b",
    "description": "Line tabulation matches",
    "valid": true
  },
  {
    "data": "\f",
    "description": "Form feed matches",
    "valid": true
  },
  {
    "data": " ",
    "description": "latin-1 non-breaking-space matches",
    "valid": true
  },
  {
    "data": "﻿",
    "description": "zero-width whitespace matches",
    "valid": true
  },
  {
    "data": "\n",
    "description": "line feed matches (line terminator)",
    "valid": true
  },
  {
    "data": " ",
    "description": "paragraph separator matches (line terminator)",
    "valid": true
  },
  {
    "data": " ",
    "description": "EM SPACE matches (Space_Separator)",
    "valid": true
  },
  {
    "data": "\u0001",
    "description": "Non-whitespace control does not match",
    "valid": false
  },
  {
    "data": "–",
    "description": "Non-whitespace does not match",
    "valid": false
  }
]
"""

from typing import ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\s$",
  "type": "string"
}
"""

_VALIDATE_FORMATS = False

class Ecmascriptregex8Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: str


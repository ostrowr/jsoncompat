"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\S$",
  "type": "string"
}

Tests:
[
  {
    "data": " ",
    "description": "ASCII space does not match",
    "valid": false
  },
  {
    "data": "\t",
    "description": "Character tabulation does not match",
    "valid": false
  },
  {
    "data": "\u000b",
    "description": "Line tabulation does not match",
    "valid": false
  },
  {
    "data": "\f",
    "description": "Form feed does not match",
    "valid": false
  },
  {
    "data": " ",
    "description": "latin-1 non-breaking-space does not match",
    "valid": false
  },
  {
    "data": "﻿",
    "description": "zero-width whitespace does not match",
    "valid": false
  },
  {
    "data": "\n",
    "description": "line feed does not match (line terminator)",
    "valid": false
  },
  {
    "data": " ",
    "description": "paragraph separator does not match (line terminator)",
    "valid": false
  },
  {
    "data": " ",
    "description": "EM SPACE does not match (Space_Separator)",
    "valid": false
  },
  {
    "data": "\u0001",
    "description": "Non-whitespace control matches",
    "valid": true
  },
  {
    "data": "–",
    "description": "Non-whitespace matches",
    "valid": true
  }
]
"""

from typing import ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\S$",
  "type": "string"
}
"""

_VALIDATE_FORMATS = False

class Ecmascriptregex9Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: str


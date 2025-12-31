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

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ecmascriptregex9Serializer(SerializerRootModel):
    root: Annotated[str, Field(pattern="^\\S$")]


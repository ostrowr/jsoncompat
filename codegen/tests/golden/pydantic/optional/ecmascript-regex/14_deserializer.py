"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\p{digit}+$"
}

Tests:
[
  {
    "data": "42",
    "description": "ascii digits",
    "valid": true
  },
  {
    "data": "-%#",
    "description": "ascii non-digits",
    "valid": false
  },
  {
    "data": "৪২",
    "description": "non-ascii digits (BENGALI DIGIT FOUR, BENGALI DIGIT TWO)",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ecmascriptregex14Deserializer(DeserializerRootModel):
    root: Annotated[str, Field(pattern="^\\p{digit}+$")]


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\W$",
  "type": "string"
}

Tests:
[
  {
    "data": "a",
    "description": "ASCII 'a' does not match",
    "valid": false
  },
  {
    "data": "é",
    "description": "latin-1 e-acute matches (unlike e.g. Python)",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ecmascriptregex7Serializer(SerializerRootModel):
    root: Annotated[str, Field(pattern="^\\W$")]


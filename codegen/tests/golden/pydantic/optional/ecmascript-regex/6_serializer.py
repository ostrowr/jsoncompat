"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\w$",
  "type": "string"
}

Tests:
[
  {
    "data": "a",
    "description": "ASCII 'a' matches",
    "valid": true
  },
  {
    "data": "é",
    "description": "latin-1 e-acute does not match (unlike e.g. Python)",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ecmascriptregex6Serializer(SerializerRootModel):
    root: Annotated[str, Field(pattern="^\\w$")]


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\cc$",
  "type": "string"
}

Tests:
[
  {
    "data": "\\cc",
    "description": "does not match",
    "valid": false
  },
  {
    "data": "\u0003",
    "description": "matches",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ecmascriptregex3Deserializer(DeserializerRootModel):
    root: Annotated[str, Field(pattern="^\u0003$")]


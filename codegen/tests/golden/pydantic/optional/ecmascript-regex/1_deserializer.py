"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\t$",
  "type": "string"
}

Tests:
[
  {
    "data": "\\t",
    "description": "does not match",
    "valid": false
  },
  {
    "data": "\t",
    "description": "matches",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ecmascriptregex1Deserializer(DeserializerRootModel):
    root: Annotated[str, Field(pattern="^\\t$")]


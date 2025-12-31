"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\D$",
  "type": "string"
}

Tests:
[
  {
    "data": "0",
    "description": "ASCII zero does not match",
    "valid": false
  },
  {
    "data": "߀",
    "description": "NKO DIGIT ZERO matches (unlike e.g. Python)",
    "valid": true
  },
  {
    "data": "߀",
    "description": "NKO DIGIT ZERO (as \\u escape) matches",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ecmascriptregex5Deserializer(DeserializerRootModel):
    root: Annotated[str, Field(pattern="^\\D$")]


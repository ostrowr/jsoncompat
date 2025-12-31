"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\d$",
  "type": "string"
}

Tests:
[
  {
    "data": "0",
    "description": "ASCII zero matches",
    "valid": true
  },
  {
    "data": "߀",
    "description": "NKO DIGIT ZERO does not match (unlike e.g. Python)",
    "valid": false
  },
  {
    "data": "߀",
    "description": "NKO DIGIT ZERO (as \\u escape) does not match",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ecmascriptregex4Serializer(SerializerRootModel):
    root: Annotated[str, Field(pattern="^\\d$")]


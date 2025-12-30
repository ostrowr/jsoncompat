"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^abc$",
  "type": "string"
}

Tests:
[
  {
    "data": "abc\\n",
    "description": "matches in Python, but not in ECMA 262",
    "valid": false
  },
  {
    "data": "abc",
    "description": "matches",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ecmascriptregex0Serializer(SerializerRootModel):
    root: Annotated[str, Field(pattern="^abc$")]


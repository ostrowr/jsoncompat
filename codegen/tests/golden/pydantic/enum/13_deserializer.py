"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    "hello\u0000there"
  ]
}

Tests:
[
  {
    "data": "hello\u0000there",
    "description": "match string with nul",
    "valid": true
  },
  {
    "data": "hellothere",
    "description": "do not match string lacking nul",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Enum13Deserializer(DeserializerRootModel):
    root: Literal["hello\u0000there"]


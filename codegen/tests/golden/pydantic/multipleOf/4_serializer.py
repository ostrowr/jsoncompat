"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "multipleOf": 1e-8,
  "type": "integer"
}

Tests:
[
  {
    "data": 12391239123,
    "description": "any integer is a multiple of 1e-8",
    "valid": true
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Multipleof4Serializer(SerializerRootModel):
    root: int


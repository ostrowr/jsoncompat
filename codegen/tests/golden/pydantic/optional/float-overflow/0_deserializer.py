"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "multipleOf": 0.5,
  "type": "integer"
}

Tests:
[
  {
    "data": 1e308,
    "description": "valid if optional overflow handling is implemented",
    "valid": true
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Floatoverflow0Deserializer(DeserializerRootModel):
    root: int


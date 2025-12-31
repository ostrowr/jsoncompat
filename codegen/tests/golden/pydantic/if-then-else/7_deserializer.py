"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "const": "else"
  },
  "if": true,
  "then": {
    "const": "then"
  }
}

Tests:
[
  {
    "data": "then",
    "description": "boolean schema true in if always chooses the then path (valid)",
    "valid": true
  },
  {
    "data": "else",
    "description": "boolean schema true in if always chooses the then path (invalid)",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ifthenelse7Deserializer(DeserializerRootModel):
    root: Any


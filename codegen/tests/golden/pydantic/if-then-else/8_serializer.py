"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "const": "else"
  },
  "if": false,
  "then": {
    "const": "then"
  }
}

Tests:
[
  {
    "data": "then",
    "description": "boolean schema false in if always chooses the else path (invalid)",
    "valid": false
  },
  {
    "data": "else",
    "description": "boolean schema false in if always chooses the else path (valid)",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ifthenelse8Serializer(SerializerRootModel):
    root: Any


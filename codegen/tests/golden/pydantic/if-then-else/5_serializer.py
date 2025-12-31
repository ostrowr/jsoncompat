"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "multipleOf": 2
  },
  "if": {
    "exclusiveMaximum": 0
  },
  "then": {
    "minimum": -10
  }
}

Tests:
[
  {
    "data": -1,
    "description": "valid through then",
    "valid": true
  },
  {
    "data": -100,
    "description": "invalid through then",
    "valid": false
  },
  {
    "data": 4,
    "description": "valid through else",
    "valid": true
  },
  {
    "data": 3,
    "description": "invalid through else",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ifthenelse5Serializer(SerializerRootModel):
    root: Any


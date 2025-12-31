"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "patternProperties": {
    "^🐲*$": {
      "type": "integer"
    }
  }
}

Tests:
[
  {
    "data": {
      "": 1
    },
    "description": "matches empty",
    "valid": true
  },
  {
    "data": {
      "🐲": 1
    },
    "description": "matches single",
    "valid": true
  },
  {
    "data": {
      "🐲🐲": 1
    },
    "description": "matches two",
    "valid": true
  },
  {
    "data": {
      "🐲": "hello"
    },
    "description": "doesn't match one",
    "valid": false
  },
  {
    "data": {
      "🐲🐲": "hello"
    },
    "description": "doesn't match two",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Nonbmpregex1Deserializer(DeserializerRootModel):
    root: Any


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "not": {
    "type": [
      "integer",
      "boolean"
    ]
  }
}

Tests:
[
  {
    "data": "foo",
    "description": "valid",
    "valid": true
  },
  {
    "data": 1,
    "description": "mismatch",
    "valid": false
  },
  {
    "data": true,
    "description": "other mismatch",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Not1Deserializer(DeserializerRootModel):
    root: Any


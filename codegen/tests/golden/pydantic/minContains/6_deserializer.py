"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contains": {
    "const": 1
  },
  "minContains": 0
}

Tests:
[
  {
    "data": [],
    "description": "empty data",
    "valid": true
  },
  {
    "data": [
      2
    ],
    "description": "minContains = 0 makes contains always pass",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Mincontains6Deserializer(DeserializerRootModel):
    root: Any


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minContains": 1
}

Tests:
[
  {
    "data": [
      1
    ],
    "description": "one item valid against lone minContains",
    "valid": true
  },
  {
    "data": [],
    "description": "zero items still valid against lone minContains",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Mincontains0Deserializer(DeserializerRootModel):
    root: Any


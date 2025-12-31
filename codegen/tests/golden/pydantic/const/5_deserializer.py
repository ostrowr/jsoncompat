"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": true
}

Tests:
[
  {
    "data": true,
    "description": "true is valid",
    "valid": true
  },
  {
    "data": 1,
    "description": "integer one is invalid",
    "valid": false
  },
  {
    "data": 1.0,
    "description": "float one is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Const5Deserializer(DeserializerRootModel):
    root: Literal[True]


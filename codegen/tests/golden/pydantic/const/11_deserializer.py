"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": 1
}

Tests:
[
  {
    "data": true,
    "description": "true is invalid",
    "valid": false
  },
  {
    "data": 1,
    "description": "integer one is valid",
    "valid": true
  },
  {
    "data": 1.0,
    "description": "float one is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Const11Deserializer(DeserializerRootModel):
    root: Literal[1]


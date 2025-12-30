"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": 2
}

Tests:
[
  {
    "data": 2,
    "description": "same value is valid",
    "valid": true
  },
  {
    "data": 5,
    "description": "another value is invalid",
    "valid": false
  },
  {
    "data": "a",
    "description": "another type is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Const0Serializer(SerializerRootModel):
    root: Literal[2]


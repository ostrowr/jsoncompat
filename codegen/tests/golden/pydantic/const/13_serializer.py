"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": 9007199254740992
}

Tests:
[
  {
    "data": 9007199254740992,
    "description": "integer is valid",
    "valid": true
  },
  {
    "data": 9007199254740991,
    "description": "integer minus one is invalid",
    "valid": false
  },
  {
    "data": 9007199254740992.0,
    "description": "float is valid",
    "valid": true
  },
  {
    "data": 9007199254740990.0,
    "description": "float minus one is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Const13Serializer(SerializerRootModel):
    root: Literal[9007199254740992]


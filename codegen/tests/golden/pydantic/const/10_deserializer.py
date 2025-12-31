"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": 0
}

Tests:
[
  {
    "data": false,
    "description": "false is invalid",
    "valid": false
  },
  {
    "data": 0,
    "description": "integer zero is valid",
    "valid": true
  },
  {
    "data": 0.0,
    "description": "float zero is valid",
    "valid": true
  },
  {
    "data": {},
    "description": "empty object is invalid",
    "valid": false
  },
  {
    "data": [],
    "description": "empty array is invalid",
    "valid": false
  },
  {
    "data": "",
    "description": "empty string is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Const10Deserializer(DeserializerRootModel):
    root: Literal[0]


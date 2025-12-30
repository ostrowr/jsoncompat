"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    1,
    2,
    3
  ]
}

Tests:
[
  {
    "data": 1,
    "description": "one of the enum is valid",
    "valid": true
  },
  {
    "data": 4,
    "description": "something else is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Enum0Serializer(SerializerRootModel):
    root: Literal[1, 2, 3]


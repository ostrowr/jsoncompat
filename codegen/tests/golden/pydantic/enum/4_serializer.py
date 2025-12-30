"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    "foo\nbar",
    "foo\rbar"
  ]
}

Tests:
[
  {
    "data": "foo\nbar",
    "description": "member 1 is valid",
    "valid": true
  },
  {
    "data": "foo\rbar",
    "description": "member 2 is valid",
    "valid": true
  },
  {
    "data": "abc",
    "description": "another string is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Enum4Serializer(SerializerRootModel):
    root: Literal["foo\nbar", "foo\rbar"]


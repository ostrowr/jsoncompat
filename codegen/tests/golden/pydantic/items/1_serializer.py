"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": true
}

Tests:
[
  {
    "data": [
      1,
      "foo",
      true
    ],
    "description": "any array is valid",
    "valid": true
  },
  {
    "data": [],
    "description": "empty array is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Items1Serializer(SerializerRootModel):
    root: list[Any]


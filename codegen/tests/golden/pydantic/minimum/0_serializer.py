"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minimum": 1.1
}

Tests:
[
  {
    "data": 2.6,
    "description": "above the minimum is valid",
    "valid": true
  },
  {
    "data": 1.1,
    "description": "boundary point is valid",
    "valid": true
  },
  {
    "data": 0.6,
    "description": "below the minimum is invalid",
    "valid": false
  },
  {
    "data": "x",
    "description": "ignores non-numbers",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Minimum0Serializer(SerializerRootModel):
    root: Annotated[float, Field(ge=1.1)]


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "exclusiveMinimum": 1.1
}

Tests:
[
  {
    "data": 1.2,
    "description": "above the exclusiveMinimum is valid",
    "valid": true
  },
  {
    "data": 1.1,
    "description": "boundary point is invalid",
    "valid": false
  },
  {
    "data": 0.6,
    "description": "below the exclusiveMinimum is invalid",
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

class Exclusiveminimum0Deserializer(DeserializerRootModel):
    root: Annotated[float, Field(gt=1.1)]


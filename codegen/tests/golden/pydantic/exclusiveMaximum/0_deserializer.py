"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "exclusiveMaximum": 3.0
}

Tests:
[
  {
    "data": 2.2,
    "description": "below the exclusiveMaximum is valid",
    "valid": true
  },
  {
    "data": 3.0,
    "description": "boundary point is invalid",
    "valid": false
  },
  {
    "data": 3.5,
    "description": "above the exclusiveMaximum is invalid",
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

class Exclusivemaximum0Deserializer(DeserializerRootModel):
    root: Annotated[float, Field(lt=3.0)]


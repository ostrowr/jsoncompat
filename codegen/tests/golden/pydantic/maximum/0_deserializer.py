"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "maximum": 3.0
}

Tests:
[
  {
    "data": 2.6,
    "description": "below the maximum is valid",
    "valid": true
  },
  {
    "data": 3.0,
    "description": "boundary point is valid",
    "valid": true
  },
  {
    "data": 3.5,
    "description": "above the maximum is invalid",
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

class Maximum0Deserializer(DeserializerRootModel):
    root: Annotated[float, Field(le=3.0)]


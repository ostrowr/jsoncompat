"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minimum": -2
}

Tests:
[
  {
    "data": -1,
    "description": "negative above the minimum is valid",
    "valid": true
  },
  {
    "data": 0,
    "description": "positive above the minimum is valid",
    "valid": true
  },
  {
    "data": -2,
    "description": "boundary point is valid",
    "valid": true
  },
  {
    "data": -2.0,
    "description": "boundary point with float is valid",
    "valid": true
  },
  {
    "data": -2.0001,
    "description": "float below the minimum is invalid",
    "valid": false
  },
  {
    "data": -3,
    "description": "int below the minimum is invalid",
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

class Minimum1Deserializer(DeserializerRootModel):
    root: Annotated[float, Field(ge=-2.0)]


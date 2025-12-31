"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "maxItems": 2
}

Tests:
[
  {
    "data": [
      1
    ],
    "description": "shorter is valid",
    "valid": true
  },
  {
    "data": [
      1,
      2
    ],
    "description": "exact length is valid",
    "valid": true
  },
  {
    "data": [
      1,
      2,
      3
    ],
    "description": "too long is invalid",
    "valid": false
  },
  {
    "data": "foobar",
    "description": "ignores non-arrays",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Maxitems0Deserializer(DeserializerRootModel):
    root: Annotated[list[Any], Field(max_length=2)]


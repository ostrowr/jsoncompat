"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "type": "integer"
    },
    {
      "minimum": 2
    }
  ]
}

Tests:
[
  {
    "data": 1,
    "description": "first anyOf valid",
    "valid": true
  },
  {
    "data": 2.5,
    "description": "second anyOf valid",
    "valid": true
  },
  {
    "data": 3,
    "description": "both anyOf valid",
    "valid": true
  },
  {
    "data": 1.5,
    "description": "neither anyOf valid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Anyof0Deserializer(DeserializerRootModel):
    root: int | Annotated[float, Field(ge=2.0)]


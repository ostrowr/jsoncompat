"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
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
    "description": "first oneOf valid",
    "valid": true
  },
  {
    "data": 2.5,
    "description": "second oneOf valid",
    "valid": true
  },
  {
    "data": 3,
    "description": "both oneOf valid",
    "valid": false
  },
  {
    "data": 1.5,
    "description": "neither oneOf valid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Oneof0Serializer(SerializerRootModel):
    root: int | Annotated[float, Field(ge=2.0)]


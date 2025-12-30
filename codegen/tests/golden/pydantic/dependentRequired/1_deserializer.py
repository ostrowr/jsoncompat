"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "dependentRequired": {
    "bar": []
  }
}

Tests:
[
  {
    "data": {},
    "description": "empty object",
    "valid": true
  },
  {
    "data": {
      "bar": 2
    },
    "description": "object with one property",
    "valid": true
  },
  {
    "data": 1,
    "description": "non-object is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Dependentrequired1Deserializer(DeserializerRootModel):
    root: Any


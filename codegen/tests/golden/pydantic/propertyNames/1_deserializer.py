"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "propertyNames": {
    "pattern": "^a+$"
  }
}

Tests:
[
  {
    "data": {
      "a": {},
      "aa": {},
      "aaa": {}
    },
    "description": "matching property names valid",
    "valid": true
  },
  {
    "data": {
      "aaA": {}
    },
    "description": "non-matching property name is invalid",
    "valid": false
  },
  {
    "data": {},
    "description": "object without properties is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Propertynames1Deserializer(DeserializerRootModel):
    root: Any


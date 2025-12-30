"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "propertyNames": true
}

Tests:
[
  {
    "data": {
      "foo": 1
    },
    "description": "object with any properties is valid",
    "valid": true
  },
  {
    "data": {},
    "description": "empty object is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Propertynames2Deserializer(DeserializerRootModel):
    root: Any


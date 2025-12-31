"""
Schema:
{
  "$ref": "http://localhost:1234/draft2020-12/locationIndependentIdentifier.json#/$defs/refToInteger",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": 1,
    "description": "integer is valid",
    "valid": true
  },
  {
    "data": "foo",
    "description": "string is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Refremote9Deserializer(DeserializerRootModel):
    root: Any


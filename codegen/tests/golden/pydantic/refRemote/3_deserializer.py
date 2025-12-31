"""
Schema:
{
  "$ref": "http://localhost:1234/draft2020-12/subSchemas.json#/$defs/refToInteger",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": 1,
    "description": "ref within ref valid",
    "valid": true
  },
  {
    "data": "a",
    "description": "ref within ref invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Refremote3Deserializer(DeserializerRootModel):
    root: Any


"""
Schema:
{
  "$ref": "http://localhost:1234/draft2020-12/detached-ref.json#/$defs/foo",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": 1,
    "description": "number is valid",
    "valid": true
  },
  {
    "data": "a",
    "description": "non-number is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Refremote14Deserializer(DeserializerRootModel):
    root: Any


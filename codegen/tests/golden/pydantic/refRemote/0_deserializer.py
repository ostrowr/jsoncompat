"""
Schema:
{
  "$ref": "http://localhost:1234/draft2020-12/integer.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": 1,
    "description": "remote ref valid",
    "valid": true
  },
  {
    "data": "a",
    "description": "remote ref invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Refremote0Deserializer(DeserializerRootModel):
    root: Any


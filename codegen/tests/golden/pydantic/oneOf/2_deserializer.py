"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    true,
    true,
    true
  ]
}

Tests:
[
  {
    "data": "foo",
    "description": "any value is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Oneof2Deserializer(DeserializerRootModel):
    root: Any


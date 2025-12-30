"""
Schema:
{
  "$defs": {
    "bool": true
  },
  "$ref": "#/$defs/bool",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": "foo",
    "description": "any value is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ref9Deserializer(DeserializerRootModel):
    root: Any


"""
Schema:
{
  "$ref": "http://localhost:1234/nested-absolute-ref-to-string.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": 1,
    "description": "number is invalid",
    "valid": false
  },
  {
    "data": "foo",
    "description": "string is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Refremote13Serializer(SerializerRootModel):
    root: Any


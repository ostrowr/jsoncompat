"""
Schema:
{
  "$ref": "http://localhost:1234/draft2020-12/locationIndependentIdentifier.json#foo",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": 1,
    "description": "remote anchor valid",
    "valid": true
  },
  {
    "data": "a",
    "description": "remote anchor invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Refremote2Serializer(SerializerRootModel):
    root: Any


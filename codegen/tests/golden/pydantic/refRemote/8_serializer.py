"""
Schema:
{
  "$id": "http://localhost:1234/draft2020-12/schema-remote-ref-ref-defs1.json",
  "$ref": "ref-and-defs.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": {
      "bar": 1
    },
    "description": "invalid",
    "valid": false
  },
  {
    "data": {
      "bar": "a"
    },
    "description": "valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Refremote8Serializer(SerializerRootModel):
    root: Any


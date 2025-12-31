"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "not": {
    "type": "integer"
  }
}

Tests:
[
  {
    "data": "foo",
    "description": "allowed",
    "valid": true
  },
  {
    "data": 1,
    "description": "disallowed",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Not0Serializer(SerializerRootModel):
    root: Any


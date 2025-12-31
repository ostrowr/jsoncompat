"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "not": {
    "properties": {
      "foo": {
        "type": "string"
      }
    },
    "type": "object"
  }
}

Tests:
[
  {
    "data": 1,
    "description": "match",
    "valid": true
  },
  {
    "data": {
      "foo": 1
    },
    "description": "other match",
    "valid": true
  },
  {
    "data": {
      "foo": "bar"
    },
    "description": "mismatch",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Not2Deserializer(DeserializerRootModel):
    root: Any


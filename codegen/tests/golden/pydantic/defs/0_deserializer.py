"""
Schema:
{
  "$ref": "https://json-schema.org/draft/2020-12/schema",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": {
      "$defs": {
        "foo": {
          "type": "integer"
        }
      }
    },
    "description": "valid definition schema",
    "valid": true
  },
  {
    "data": {
      "$defs": {
        "foo": {
          "type": 1
        }
      }
    },
    "description": "invalid definition schema",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Defs0Deserializer(DeserializerRootModel):
    root: Any


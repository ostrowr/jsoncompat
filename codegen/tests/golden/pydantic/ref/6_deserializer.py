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
      "minLength": 1
    },
    "description": "remote ref valid",
    "valid": true
  },
  {
    "data": {
      "minLength": -1
    },
    "description": "remote ref invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ref6Deserializer(DeserializerRootModel):
    root: Any


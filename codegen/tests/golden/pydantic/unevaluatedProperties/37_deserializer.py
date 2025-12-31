"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "propertyNames": {
    "maxLength": 1
  },
  "unevaluatedProperties": {
    "type": "number"
  }
}

Tests:
[
  {
    "data": {
      "a": 1
    },
    "description": "allows only number properties",
    "valid": true
  },
  {
    "data": {
      "a": "b"
    },
    "description": "string property is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Unevaluatedproperties37Deserializer(DeserializerRootModel):
    root: Any


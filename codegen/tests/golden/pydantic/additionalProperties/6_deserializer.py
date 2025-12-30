"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "type": "null"
  }
}

Tests:
[
  {
    "data": {
      "foo": null
    },
    "description": "allows null values",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Additionalproperties6Deserializer(DeserializerRootModel):
    root: Any


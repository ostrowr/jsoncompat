"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "patternProperties": {
    "^.*bar$": {
      "type": "null"
    }
  }
}

Tests:
[
  {
    "data": {
      "foobar": null
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

class Patternproperties4Deserializer(DeserializerRootModel):
    root: Any


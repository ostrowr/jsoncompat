"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contains": {
    "type": "null"
  }
}

Tests:
[
  {
    "data": [
      null
    ],
    "description": "allows null items",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Contains6Serializer(SerializerRootModel):
    root: Any


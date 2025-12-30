"""
Schema:
{
  "$ref": "http://localhost:1234/draft2019-09/ignore-prefixItems.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "array"
}

Tests:
[
  {
    "comment": "if the implementation is not processing the $ref as a 2019-09 schema, this test will fail",
    "data": [
      1,
      2,
      3
    ],
    "description": "first item not a string is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Crossdraft0Serializer(SerializerRootModel):
    root: Any


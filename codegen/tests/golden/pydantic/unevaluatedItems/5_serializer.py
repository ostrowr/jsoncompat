"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": true,
  "prefixItems": [
    {
      "type": "string"
    }
  ],
  "unevaluatedItems": false
}

Tests:
[
  {
    "data": [
      "foo",
      42
    ],
    "description": "unevaluatedItems doesn't apply",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Unevaluateditems5Serializer(SerializerRootModel):
    root: list[Any]


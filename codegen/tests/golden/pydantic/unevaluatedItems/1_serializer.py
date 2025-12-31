"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "unevaluatedItems": false
}

Tests:
[
  {
    "data": [],
    "description": "with no unevaluated items",
    "valid": true
  },
  {
    "data": [
      "foo"
    ],
    "description": "with unevaluated items",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Unevaluateditems1Serializer(SerializerRootModel):
    root: Any


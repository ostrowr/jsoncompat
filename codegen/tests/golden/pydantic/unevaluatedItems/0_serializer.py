"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "unevaluatedItems": true
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
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Unevaluateditems0Serializer(SerializerRootModel):
    root: Any


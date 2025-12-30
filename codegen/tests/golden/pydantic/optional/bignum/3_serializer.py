"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "maximum": 18446744073709551615
}

Tests:
[
  {
    "data": 18446744073709551600,
    "description": "comparison works for high numbers",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Bignum3Serializer(SerializerRootModel):
    root: Any


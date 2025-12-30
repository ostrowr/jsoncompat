"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minimum": -1.8446744073709552e19
}

Tests:
[
  {
    "data": -1.8446744073709552e19,
    "description": "comparison works for very negative numbers",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Bignum5Serializer(SerializerRootModel):
    root: Any


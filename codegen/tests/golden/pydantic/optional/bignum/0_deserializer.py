"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "integer"
}

Tests:
[
  {
    "data": 1.2345678910111214e52,
    "description": "a bignum is an integer",
    "valid": true
  },
  {
    "data": -1.2345678910111214e52,
    "description": "a negative bignum is an integer",
    "valid": true
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Bignum0Deserializer(DeserializerRootModel):
    root: int


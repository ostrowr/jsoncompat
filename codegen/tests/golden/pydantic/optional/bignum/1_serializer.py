"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "number"
}

Tests:
[
  {
    "data": 9.82492837492349e52,
    "description": "a bignum is a number",
    "valid": true
  },
  {
    "data": -9.82492837492349e52,
    "description": "a negative bignum is a number",
    "valid": true
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Bignum1Serializer(SerializerRootModel):
    root: float


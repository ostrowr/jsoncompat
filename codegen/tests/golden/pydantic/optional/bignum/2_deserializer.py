"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "string"
}

Tests:
[
  {
    "data": 9.82492837492349e52,
    "description": "a bignum is not a string",
    "valid": false
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Bignum2Deserializer(DeserializerRootModel):
    root: str


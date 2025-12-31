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

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Bignum5Deserializer(DeserializerRootModel):
    root: Annotated[float, Field(ge=-18446744073709552000.0)]


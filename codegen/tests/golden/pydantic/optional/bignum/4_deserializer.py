"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "exclusiveMaximum": 9.727837981879871e26
}

Tests:
[
  {
    "data": 9.727837981879871e26,
    "description": "comparison works for high numbers",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Bignum4Deserializer(DeserializerRootModel):
    root: Annotated[float, Field(lt=972783798187987100000000000.0)]


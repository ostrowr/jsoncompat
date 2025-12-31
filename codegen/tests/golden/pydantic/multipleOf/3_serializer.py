"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "multipleOf": 0.123456789,
  "type": "integer"
}

Tests:
[
  {
    "data": 1e308,
    "description": "always invalid, but naive implementations may raise an overflow error",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Multipleof3Serializer(SerializerRootModel):
    root: Annotated[int, Field(multiple_of=0.123456789)]


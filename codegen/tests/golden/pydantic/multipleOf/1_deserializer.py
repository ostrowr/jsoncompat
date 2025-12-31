"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "multipleOf": 1.5
}

Tests:
[
  {
    "data": 0,
    "description": "zero is multiple of anything",
    "valid": true
  },
  {
    "data": 4.5,
    "description": "4.5 is multiple of 1.5",
    "valid": true
  },
  {
    "data": 35,
    "description": "35 is not multiple of 1.5",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Multipleof1Deserializer(DeserializerRootModel):
    root: Annotated[float, Field(multiple_of=1.5)]


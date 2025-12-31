"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "multipleOf": 0.0001
}

Tests:
[
  {
    "data": 0.0075,
    "description": "0.0075 is multiple of 0.0001",
    "valid": true
  },
  {
    "data": 0.00751,
    "description": "0.00751 is not multiple of 0.0001",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Multipleof2Serializer(SerializerRootModel):
    root: Annotated[float, Field(multiple_of=0.0001)]


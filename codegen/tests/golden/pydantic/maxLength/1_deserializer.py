"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "maxLength": 2
}

Tests:
[
  {
    "data": "f",
    "description": "shorter is valid",
    "valid": true
  },
  {
    "data": "foo",
    "description": "too long is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Maxlength1Deserializer(DeserializerRootModel):
    root: Annotated[str, Field(max_length=2)]


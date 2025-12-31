"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    "hello\u0000there"
  ]
}

Tests:
[
  {
    "data": "hello\u0000there",
    "description": "match string with nul",
    "valid": true
  },
  {
    "data": "hellothere",
    "description": "do not match string lacking nul",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

class Enum13Deserializer(DeserializerRootModel):
    root: Annotated[Literal["hello\u0000there"], BeforeValidator(lambda v, _allowed=["hello\u0000there"]: _validate_literal(v, _allowed))]


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": null
}

Tests:
[
  {
    "data": null,
    "description": "null is valid",
    "valid": true
  },
  {
    "data": 0,
    "description": "not null is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

class Const3Serializer(SerializerRootModel):
    root: Annotated[Literal[None], BeforeValidator(lambda v, _allowed=[None]: _validate_literal(v, _allowed))]


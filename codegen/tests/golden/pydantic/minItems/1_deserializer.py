"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minItems": 1
}

Tests:
[
  {
    "data": [
      1,
      2
    ],
    "description": "longer is valid",
    "valid": true
  },
  {
    "data": [],
    "description": "too short is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter
from pydantic.functional_validators import BeforeValidator

class Minitems1Deserializer(DeserializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _adapter=TypeAdapter(Annotated[list[Any], Field(min_length=1)], config=ConfigDict(strict=True)): v if not isinstance(v, list) else _adapter.validate_python(v))]


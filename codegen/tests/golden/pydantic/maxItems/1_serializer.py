"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "maxItems": 2
}

Tests:
[
  {
    "data": [
      1
    ],
    "description": "shorter is valid",
    "valid": true
  },
  {
    "data": [
      1,
      2,
      3
    ],
    "description": "too long is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter
from pydantic.functional_validators import BeforeValidator

class Maxitems1Serializer(SerializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _adapter=TypeAdapter(Annotated[list[Any], Field(max_length=2)], config=ConfigDict(strict=True)): v if not isinstance(v, list) else _adapter.validate_python(v))]


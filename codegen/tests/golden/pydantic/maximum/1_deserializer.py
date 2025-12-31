"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "maximum": 300
}

Tests:
[
  {
    "data": 299.97,
    "description": "below the maximum is invalid",
    "valid": true
  },
  {
    "data": 300,
    "description": "boundary point integer is valid",
    "valid": true
  },
  {
    "data": 300.0,
    "description": "boundary point float is valid",
    "valid": true
  },
  {
    "data": 300.5,
    "description": "above the maximum is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter
from pydantic.functional_validators import BeforeValidator

class Maximum1Deserializer(DeserializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _adapter=TypeAdapter(Annotated[float, Field(le=300.0)], config=ConfigDict(strict=True)): v if isinstance(v, bool) or not isinstance(v, (int, float)) else _adapter.validate_python(v))]


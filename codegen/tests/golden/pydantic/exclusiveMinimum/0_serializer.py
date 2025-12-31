"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "exclusiveMinimum": 1.1
}

Tests:
[
  {
    "data": 1.2,
    "description": "above the exclusiveMinimum is valid",
    "valid": true
  },
  {
    "data": 1.1,
    "description": "boundary point is invalid",
    "valid": false
  },
  {
    "data": 0.6,
    "description": "below the exclusiveMinimum is invalid",
    "valid": false
  },
  {
    "data": "x",
    "description": "ignores non-numbers",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter
from pydantic.functional_validators import BeforeValidator

class Exclusiveminimum0Serializer(SerializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _adapter=TypeAdapter(Annotated[float, Field(gt=1.1)], config=ConfigDict(strict=True)): v if isinstance(v, bool) or not isinstance(v, (int, float)) else _adapter.validate_python(v))]


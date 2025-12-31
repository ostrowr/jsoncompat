"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "maximum": 3.0
}

Tests:
[
  {
    "data": 2.6,
    "description": "below the maximum is valid",
    "valid": true
  },
  {
    "data": 3.0,
    "description": "boundary point is valid",
    "valid": true
  },
  {
    "data": 3.5,
    "description": "above the maximum is invalid",
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

class Maximum0Deserializer(DeserializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _adapter=TypeAdapter(Annotated[float, Field(le=3.0)], config=ConfigDict(strict=True)): v if isinstance(v, bool) or not isinstance(v, (int, float)) else _adapter.validate_python(v))]


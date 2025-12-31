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

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter
from pydantic.functional_validators import BeforeValidator

class Multipleof1Serializer(SerializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _adapter=TypeAdapter(Annotated[float, Field(multiple_of=1.5)], config=ConfigDict(strict=True)): v if isinstance(v, bool) or not isinstance(v, (int, float)) else _adapter.validate_python(v))]


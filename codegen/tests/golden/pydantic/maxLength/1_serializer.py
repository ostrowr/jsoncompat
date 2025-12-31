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

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter
from pydantic.functional_validators import BeforeValidator

class Maxlength1Serializer(SerializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _adapter=TypeAdapter(Annotated[str, Field(max_length=2)], config=ConfigDict(strict=True)): v if not isinstance(v, str) else _adapter.validate_python(v))]


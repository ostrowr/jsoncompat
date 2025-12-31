"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "multipleOf": 2
}

Tests:
[
  {
    "data": 10,
    "description": "int by int",
    "valid": true
  },
  {
    "data": 7,
    "description": "int by int fail",
    "valid": false
  },
  {
    "data": "foo",
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

class Multipleof0Deserializer(DeserializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _adapter=TypeAdapter(Annotated[float, Field(multiple_of=2.0)], config=ConfigDict(strict=True)): v if isinstance(v, bool) or not isinstance(v, (int, float)) else _adapter.validate_python(v))]


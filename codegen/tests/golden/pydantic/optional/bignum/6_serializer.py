"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "exclusiveMinimum": -9.727837981879871e26
}

Tests:
[
  {
    "data": -9.727837981879871e26,
    "description": "comparison works for very negative numbers",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter
from pydantic.functional_validators import BeforeValidator

class Bignum6Serializer(SerializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _adapter=TypeAdapter(Annotated[float, Field(gt=-972783798187987100000000000.0)], config=ConfigDict(strict=True)): v if isinstance(v, bool) or not isinstance(v, (int, float)) else _adapter.validate_python(v))]


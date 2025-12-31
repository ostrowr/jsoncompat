"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "a+"
}

Tests:
[
  {
    "data": "xxaayy",
    "description": "matches a substring",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter
from pydantic.functional_validators import BeforeValidator

class Pattern1Serializer(SerializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _adapter=TypeAdapter(Annotated[str, Field(pattern="a+")], config=ConfigDict(strict=True)): v if not isinstance(v, str) else _adapter.validate_python(v))]


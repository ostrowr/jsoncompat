"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^🐲*$"
}

Tests:
[
  {
    "data": "",
    "description": "matches empty",
    "valid": true
  },
  {
    "data": "🐲",
    "description": "matches single",
    "valid": true
  },
  {
    "data": "🐲🐲",
    "description": "matches two",
    "valid": true
  },
  {
    "data": "🐉",
    "description": "doesn't match one",
    "valid": false
  },
  {
    "data": "🐉🐉",
    "description": "doesn't match two",
    "valid": false
  },
  {
    "data": "D",
    "description": "doesn't match one ASCII",
    "valid": false
  },
  {
    "data": "DD",
    "description": "doesn't match two ASCII",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter
from pydantic.functional_validators import BeforeValidator

class Nonbmpregex0Serializer(SerializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _adapter=TypeAdapter(Annotated[str, Field(pattern="^🐲*$")], config=ConfigDict(strict=True)): v if not isinstance(v, str) else _adapter.validate_python(v))]


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

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter, model_validator
from pydantic.functional_validators import BeforeValidator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^🐲*$"
}
"""

_VALIDATE_FORMATS = False

class Nonbmpregex0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


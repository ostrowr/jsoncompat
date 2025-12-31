"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "multipleOf": 2
  },
  "if": {
    "exclusiveMaximum": 0
  }
}

Tests:
[
  {
    "data": -1,
    "description": "valid when if test passes",
    "valid": true
  },
  {
    "data": 4,
    "description": "valid through else",
    "valid": true
  },
  {
    "data": 3,
    "description": "invalid through else",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "multipleOf": 2
  },
  "if": {
    "exclusiveMaximum": 0
  }
}
"""

_VALIDATE_FORMATS = False

class Ifthenelse4Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


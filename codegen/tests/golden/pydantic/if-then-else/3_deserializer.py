"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "if": {
    "exclusiveMaximum": 0
  },
  "then": {
    "minimum": -10
  }
}

Tests:
[
  {
    "data": -1,
    "description": "valid through then",
    "valid": true
  },
  {
    "data": -100,
    "description": "invalid through then",
    "valid": false
  },
  {
    "data": 3,
    "description": "valid when if test fails",
    "valid": true
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "if": {
    "exclusiveMaximum": 0
  },
  "then": {
    "minimum": -10
  }
}
"""

_VALIDATE_FORMATS = False

class Ifthenelse3Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


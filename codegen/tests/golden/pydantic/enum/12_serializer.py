"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    [
      1
    ]
  ]
}

Tests:
[
  {
    "data": [
      true
    ],
    "description": "[true] is invalid",
    "valid": false
  },
  {
    "data": [
      1
    ],
    "description": "[1] is valid",
    "valid": true
  },
  {
    "data": [
      1.0
    ],
    "description": "[1.0] is valid",
    "valid": true
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field, model_validator
from pydantic.functional_validators import BeforeValidator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    [
      1
    ]
  ]
}
"""

_VALIDATE_FORMATS = False

class Enum12Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


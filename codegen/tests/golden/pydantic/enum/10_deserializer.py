"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    [
      0
    ]
  ]
}

Tests:
[
  {
    "data": [
      false
    ],
    "description": "[false] is invalid",
    "valid": false
  },
  {
    "data": [
      0
    ],
    "description": "[0] is valid",
    "valid": true
  },
  {
    "data": [
      0.0
    ],
    "description": "[0.0] is valid",
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
      0
    ]
  ]
}
"""

_VALIDATE_FORMATS = False

class Enum10Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


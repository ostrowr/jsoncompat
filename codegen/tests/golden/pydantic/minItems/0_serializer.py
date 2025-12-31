"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minItems": 1
}

Tests:
[
  {
    "data": [
      1,
      2
    ],
    "description": "longer is valid",
    "valid": true
  },
  {
    "data": [
      1
    ],
    "description": "exact length is valid",
    "valid": true
  },
  {
    "data": [],
    "description": "too short is invalid",
    "valid": false
  },
  {
    "data": "",
    "description": "ignores non-arrays",
    "valid": true
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
  "minItems": 1
}
"""

_VALIDATE_FORMATS = False

class Minitems0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


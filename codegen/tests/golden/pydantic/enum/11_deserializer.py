"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    1
  ]
}

Tests:
[
  {
    "data": true,
    "description": "true is invalid",
    "valid": false
  },
  {
    "data": 1,
    "description": "integer one is valid",
    "valid": true
  },
  {
    "data": 1.0,
    "description": "float one is valid",
    "valid": true
  }
]
"""

from typing import ClassVar, Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field, model_validator
from pydantic.functional_validators import BeforeValidator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    1
  ]
}
"""

_VALIDATE_FORMATS = False

class Enum11Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Literal[1]


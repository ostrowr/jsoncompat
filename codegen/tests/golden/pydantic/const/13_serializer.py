"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": 9007199254740992
}

Tests:
[
  {
    "data": 9007199254740992,
    "description": "integer is valid",
    "valid": true
  },
  {
    "data": 9007199254740991,
    "description": "integer minus one is invalid",
    "valid": false
  },
  {
    "data": 9007199254740992.0,
    "description": "float is valid",
    "valid": true
  },
  {
    "data": 9007199254740990.0,
    "description": "float minus one is invalid",
    "valid": false
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
  "const": 9007199254740992
}
"""

_VALIDATE_FORMATS = False

class Const13Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Literal[9007199254740992]


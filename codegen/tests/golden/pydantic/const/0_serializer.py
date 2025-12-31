"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": 2
}

Tests:
[
  {
    "data": 2,
    "description": "same value is valid",
    "valid": true
  },
  {
    "data": 5,
    "description": "another value is invalid",
    "valid": false
  },
  {
    "data": "a",
    "description": "another type is invalid",
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
  "const": 2
}
"""

_VALIDATE_FORMATS = False

class Const0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Literal[2]


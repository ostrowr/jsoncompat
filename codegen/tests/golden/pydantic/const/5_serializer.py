"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": true
}

Tests:
[
  {
    "data": true,
    "description": "true is valid",
    "valid": true
  },
  {
    "data": 1,
    "description": "integer one is invalid",
    "valid": false
  },
  {
    "data": 1.0,
    "description": "float one is invalid",
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
  "const": true
}
"""

_VALIDATE_FORMATS = False

class Const5Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Literal[True]


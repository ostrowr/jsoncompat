"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minimum": -2
}

Tests:
[
  {
    "data": -1,
    "description": "negative above the minimum is valid",
    "valid": true
  },
  {
    "data": 0,
    "description": "positive above the minimum is valid",
    "valid": true
  },
  {
    "data": -2,
    "description": "boundary point is valid",
    "valid": true
  },
  {
    "data": -2.0,
    "description": "boundary point with float is valid",
    "valid": true
  },
  {
    "data": -2.0001,
    "description": "float below the minimum is invalid",
    "valid": false
  },
  {
    "data": -3,
    "description": "int below the minimum is invalid",
    "valid": false
  },
  {
    "data": "x",
    "description": "ignores non-numbers",
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
  "minimum": -2
}
"""

_VALIDATE_FORMATS = False

class Minimum1Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


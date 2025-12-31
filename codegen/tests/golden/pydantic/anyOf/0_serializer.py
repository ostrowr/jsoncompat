"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "type": "integer"
    },
    {
      "minimum": 2
    }
  ]
}

Tests:
[
  {
    "data": 1,
    "description": "first anyOf valid",
    "valid": true
  },
  {
    "data": 2.5,
    "description": "second anyOf valid",
    "valid": true
  },
  {
    "data": 3,
    "description": "both anyOf valid",
    "valid": true
  },
  {
    "data": 1.5,
    "description": "neither anyOf valid",
    "valid": false
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
  "anyOf": [
    {
      "type": "integer"
    },
    {
      "minimum": 2
    }
  ]
}
"""

_VALIDATE_FORMATS = False

class Anyof0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: int | float | Any


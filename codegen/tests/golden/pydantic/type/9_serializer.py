"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": [
    "array",
    "object"
  ]
}

Tests:
[
  {
    "data": [
      1,
      2,
      3
    ],
    "description": "array is valid",
    "valid": true
  },
  {
    "data": {
      "foo": 123
    },
    "description": "object is valid",
    "valid": true
  },
  {
    "data": 123,
    "description": "number is invalid",
    "valid": false
  },
  {
    "data": "foo",
    "description": "string is invalid",
    "valid": false
  },
  {
    "data": null,
    "description": "null is invalid",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": [
    "array",
    "object"
  ]
}
"""

_VALIDATE_FORMATS = False

class ModelSerializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": [
    "array",
    "object"
  ]
}
"""
    model_config = ConfigDict(extra="allow")

class Type9Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: list[Any] | ModelSerializer


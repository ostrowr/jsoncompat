"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": [
    "integer",
    "string"
  ]
}

Tests:
[
  {
    "data": 1,
    "description": "an integer is valid",
    "valid": true
  },
  {
    "data": "foo",
    "description": "a string is valid",
    "valid": true
  },
  {
    "data": 1.1,
    "description": "a float is invalid",
    "valid": false
  },
  {
    "data": {},
    "description": "an object is invalid",
    "valid": false
  },
  {
    "data": [],
    "description": "an array is invalid",
    "valid": false
  },
  {
    "data": true,
    "description": "a boolean is invalid",
    "valid": false
  },
  {
    "data": null,
    "description": "null is invalid",
    "valid": false
  }
]
"""

from typing import ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": [
    "integer",
    "string"
  ]
}
"""

_VALIDATE_FORMATS = False

class Type7Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: int | str


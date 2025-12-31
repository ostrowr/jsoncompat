"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "boolean"
}

Tests:
[
  {
    "data": 1,
    "description": "an integer is not a boolean",
    "valid": false
  },
  {
    "data": 0,
    "description": "zero is not a boolean",
    "valid": false
  },
  {
    "data": 1.1,
    "description": "a float is not a boolean",
    "valid": false
  },
  {
    "data": "foo",
    "description": "a string is not a boolean",
    "valid": false
  },
  {
    "data": "",
    "description": "an empty string is not a boolean",
    "valid": false
  },
  {
    "data": {},
    "description": "an object is not a boolean",
    "valid": false
  },
  {
    "data": [],
    "description": "an array is not a boolean",
    "valid": false
  },
  {
    "data": true,
    "description": "true is a boolean",
    "valid": true
  },
  {
    "data": false,
    "description": "false is a boolean",
    "valid": true
  },
  {
    "data": null,
    "description": "null is not a boolean",
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
  "type": "boolean"
}
"""

_VALIDATE_FORMATS = False

class Type5Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: bool


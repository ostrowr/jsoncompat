"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "string"
}

Tests:
[
  {
    "data": 1,
    "description": "1 is not a string",
    "valid": false
  },
  {
    "data": 1.1,
    "description": "a float is not a string",
    "valid": false
  },
  {
    "data": "foo",
    "description": "a string is a string",
    "valid": true
  },
  {
    "data": "1",
    "description": "a string is still a string, even if it looks like a number",
    "valid": true
  },
  {
    "data": "",
    "description": "an empty string is still a string",
    "valid": true
  },
  {
    "data": {},
    "description": "an object is not a string",
    "valid": false
  },
  {
    "data": [],
    "description": "an array is not a string",
    "valid": false
  },
  {
    "data": true,
    "description": "a boolean is not a string",
    "valid": false
  },
  {
    "data": null,
    "description": "null is not a string",
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
  "type": "string"
}
"""

_VALIDATE_FORMATS = False

class Type2Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: str


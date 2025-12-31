"""
Schema:
{
  "$defs": {
    "a": {
      "type": "integer"
    },
    "b": {
      "$ref": "#/$defs/a"
    },
    "c": {
      "$ref": "#/$defs/b"
    }
  },
  "$ref": "#/$defs/c",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": 5,
    "description": "nested ref valid",
    "valid": true
  },
  {
    "data": "a",
    "description": "nested ref invalid",
    "valid": false
  }
]
"""

from typing import ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$defs": {
    "a": {
      "type": "integer"
    },
    "b": {
      "$ref": "#/$defs/a"
    },
    "c": {
      "$ref": "#/$defs/b"
    }
  },
  "$ref": "#/$defs/c",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""

_VALIDATE_FORMATS = False

class Ref4Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: int


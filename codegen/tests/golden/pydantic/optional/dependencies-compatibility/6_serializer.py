"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "dependencies": {
    "foo\tbar": {
      "minProperties": 4
    },
    "foo'bar": {
      "required": [
        "foo\"bar"
      ]
    }
  }
}

Tests:
[
  {
    "data": {
      "a": 2,
      "b": 3,
      "c": 4,
      "foo\tbar": 1
    },
    "description": "quoted tab",
    "valid": true
  },
  {
    "data": {
      "foo'bar": {
        "foo\"bar": 1
      }
    },
    "description": "quoted quote",
    "valid": false
  },
  {
    "data": {
      "a": 2,
      "foo\tbar": 1
    },
    "description": "quoted tab invalid under dependent schema",
    "valid": false
  },
  {
    "data": {
      "foo'bar": 1
    },
    "description": "quoted quote invalid under dependent schema",
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
  "dependencies": {
    "foo\tbar": {
      "minProperties": 4
    },
    "foo'bar": {
      "required": [
        "foo\"bar"
      ]
    }
  }
}
"""

_VALIDATE_FORMATS = False

class Dependenciescompatibility6Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


"""
Schema:
{
  "$defs": {
    "elements": {
      "$dynamicAnchor": "elements",
      "additionalProperties": false,
      "properties": {
        "a": true
      },
      "required": [
        "a"
      ]
    }
  },
  "$id": "http://localhost:1234/draft2020-12/strict-extendible.json",
  "$ref": "extendible-dynamic-ref.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": {
      "a": true
    },
    "description": "incorrect parent schema",
    "valid": false
  },
  {
    "data": {
      "elements": [
        {
          "b": 1
        }
      ]
    },
    "description": "incorrect extended schema",
    "valid": false
  },
  {
    "data": {
      "elements": [
        {
          "a": 1
        }
      ]
    },
    "description": "correct extended schema",
    "valid": true
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$defs": {
    "elements": {
      "$dynamicAnchor": "elements",
      "additionalProperties": false,
      "properties": {
        "a": true
      },
      "required": [
        "a"
      ]
    }
  },
  "$id": "http://localhost:1234/draft2020-12/strict-extendible.json",
  "$ref": "extendible-dynamic-ref.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""

_VALIDATE_FORMATS = False

class Dynamicref14Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


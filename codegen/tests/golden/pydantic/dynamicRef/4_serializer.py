"""
Schema:
{
  "$defs": {
    "foo": {
      "$dynamicAnchor": "items",
      "type": "string"
    },
    "list": {
      "$defs": {
        "items": {
          "$comment": "This is only needed to satisfy the bookending requirement",
          "$dynamicAnchor": "items",
          "type": "number"
        }
      },
      "$id": "list",
      "items": {
        "$dynamicRef": "#/$defs/items"
      },
      "type": "array"
    }
  },
  "$id": "https://test.json-schema.org/dynamicRef-without-anchor/root",
  "$ref": "list",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": [
      "foo",
      "bar"
    ],
    "description": "An array of strings is invalid",
    "valid": false
  },
  {
    "data": [
      24,
      42
    ],
    "description": "An array of numbers is valid",
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
    "foo": {
      "$dynamicAnchor": "items",
      "type": "string"
    },
    "list": {
      "$defs": {
        "items": {
          "$comment": "This is only needed to satisfy the bookending requirement",
          "$dynamicAnchor": "items",
          "type": "number"
        }
      },
      "$id": "list",
      "items": {
        "$dynamicRef": "#/$defs/items"
      },
      "type": "array"
    }
  },
  "$id": "https://test.json-schema.org/dynamicRef-without-anchor/root",
  "$ref": "list",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""

_VALIDATE_FORMATS = False

class Dynamicref4Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: list[Any]


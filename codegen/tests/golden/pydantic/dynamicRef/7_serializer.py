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
          "$anchor": "items",
          "$comment": "This is only needed to give the reference somewhere to resolve to when it behaves like $ref"
        }
      },
      "$id": "list",
      "items": {
        "$dynamicRef": "#items"
      },
      "type": "array"
    }
  },
  "$id": "https://test.json-schema.org/dynamic-resolution-without-bookend/root",
  "$ref": "list",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": [
      "foo",
      42
    ],
    "description": "Any array is valid",
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
          "$anchor": "items",
          "$comment": "This is only needed to give the reference somewhere to resolve to when it behaves like $ref"
        }
      },
      "$id": "list",
      "items": {
        "$dynamicRef": "#items"
      },
      "type": "array"
    }
  },
  "$id": "https://test.json-schema.org/dynamic-resolution-without-bookend/root",
  "$ref": "list",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""

_VALIDATE_FORMATS = False

class Dynamicref7Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: list[Any]


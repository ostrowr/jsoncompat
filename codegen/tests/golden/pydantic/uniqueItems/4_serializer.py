"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "prefixItems": [
    {
      "type": "boolean"
    },
    {
      "type": "boolean"
    }
  ],
  "uniqueItems": false
}

Tests:
[
  {
    "data": [
      false,
      true
    ],
    "description": "[false, true] from items array is valid",
    "valid": true
  },
  {
    "data": [
      true,
      false
    ],
    "description": "[true, false] from items array is valid",
    "valid": true
  },
  {
    "data": [
      false,
      false
    ],
    "description": "[false, false] from items array is valid",
    "valid": true
  },
  {
    "data": [
      true,
      true
    ],
    "description": "[true, true] from items array is valid",
    "valid": true
  },
  {
    "data": [
      false,
      true,
      "foo",
      "bar"
    ],
    "description": "unique array extended from [false, true] is valid",
    "valid": true
  },
  {
    "data": [
      true,
      false,
      "foo",
      "bar"
    ],
    "description": "unique array extended from [true, false] is valid",
    "valid": true
  },
  {
    "data": [
      false,
      true,
      "foo",
      "foo"
    ],
    "description": "non-unique array extended from [false, true] is valid",
    "valid": true
  },
  {
    "data": [
      true,
      false,
      "foo",
      "foo"
    ],
    "description": "non-unique array extended from [true, false] is valid",
    "valid": true
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "prefixItems": [
    {
      "type": "boolean"
    },
    {
      "type": "boolean"
    }
  ],
  "uniqueItems": false
}
"""

_VALIDATE_FORMATS = False

class Uniqueitems4Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


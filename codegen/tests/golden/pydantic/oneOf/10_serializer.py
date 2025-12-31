"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "oneOf": [
        {
          "type": "null"
        }
      ]
    }
  ]
}

Tests:
[
  {
    "data": null,
    "description": "null is valid",
    "valid": true
  },
  {
    "data": 123,
    "description": "anything non-null is invalid",
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
  "oneOf": [
    {
      "oneOf": [
        {
          "type": "null"
        }
      ]
    }
  ]
}
"""

_VALIDATE_FORMATS = False

class Oneof10Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: None


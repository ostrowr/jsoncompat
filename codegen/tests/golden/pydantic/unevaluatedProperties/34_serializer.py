from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = False

class Unevaluatedproperties34Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "one": {
      "oneOf": [
        {
          "$ref": "#/$defs/two"
        },
        {
          "properties": {
            "b": true
          },
          "required": [
            "b"
          ]
        },
        {
          "patternProperties": {
            "x": true
          },
          "required": [
            "xx"
          ]
        },
        {
          "required": [
            "all"
          ],
          "unevaluatedProperties": true
        }
      ]
    },
    "two": {
      "oneOf": [
        {
          "properties": {
            "c": true
          },
          "required": [
            "c"
          ]
        },
        {
          "properties": {
            "d": true
          },
          "required": [
            "d"
          ]
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "$ref": "#/$defs/one"
    },
    {
      "properties": {
        "a": true
      },
      "required": [
        "a"
      ]
    }
  ],
  "unevaluatedProperties": false
}
"""
    root: Any


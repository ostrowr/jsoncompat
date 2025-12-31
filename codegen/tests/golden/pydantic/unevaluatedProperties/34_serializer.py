from typing import ClassVar

from json_schema_codegen_base import SerializerBase, DeserializerBase
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Unevaluatedproperties34Serializer(SerializerBase):
    __json_schema__: ClassVar[str] = r"""
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
    _validate_formats: ClassVar[bool] = _VALIDATE_FORMATS
    model_config = ConfigDict(extra="forbid")
    __json_compat_error__: ClassVar[str] = "unsupported schema feature at #/allOf/0: allOf with non-object schema"

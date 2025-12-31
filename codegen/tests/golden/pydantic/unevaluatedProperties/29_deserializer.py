from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Unevaluatedproperties29Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "properties": {
        "foo": {
          "properties": {
            "faz": {
              "type": "string"
            }
          }
        }
      }
    }
  ],
  "properties": {
    "foo": {
      "properties": {
        "bar": {
          "type": "string"
        }
      },
      "type": "object",
      "unevaluatedProperties": false
    }
  },
  "type": "object"
}
"""
    root: Any


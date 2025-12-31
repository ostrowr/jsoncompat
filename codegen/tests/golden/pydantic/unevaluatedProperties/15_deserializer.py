from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Unevaluatedproperties15Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "if": {
    "properties": {
      "foo": {
        "const": "then"
      }
    },
    "required": [
      "foo"
    ]
  },
  "then": {
    "properties": {
      "bar": {
        "type": "string"
      }
    },
    "required": [
      "bar"
    ]
  },
  "type": "object",
  "unevaluatedProperties": false
}
"""
    root: Any


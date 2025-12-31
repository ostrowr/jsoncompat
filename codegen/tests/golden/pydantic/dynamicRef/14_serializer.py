from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Dynamicref14Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
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
    root: Any


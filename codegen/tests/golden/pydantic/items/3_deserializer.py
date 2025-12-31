from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = False

class Items3Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "item": {
      "items": false,
      "prefixItems": [
        {
          "$ref": "#/$defs/sub-item"
        },
        {
          "$ref": "#/$defs/sub-item"
        }
      ],
      "type": "array"
    },
    "sub-item": {
      "required": [
        "foo"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": false,
  "prefixItems": [
    {
      "$ref": "#/$defs/item"
    },
    {
      "$ref": "#/$defs/item"
    },
    {
      "$ref": "#/$defs/item"
    }
  ],
  "type": "array"
}
"""
    root: list[Any]


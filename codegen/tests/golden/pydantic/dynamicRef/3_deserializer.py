from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = False

class Dynamicref3Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
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
          "$dynamicAnchor": "items"
        }
      },
      "$id": "list",
      "items": {
        "$dynamicRef": "#items"
      },
      "type": "array"
    }
  },
  "$id": "https://test.json-schema.org/typical-dynamic-resolution/root",
  "$ref": "list",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""
    root: list[Any]


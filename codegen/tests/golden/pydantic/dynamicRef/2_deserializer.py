from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Dynamicref2Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "foo": {
      "$dynamicAnchor": "items",
      "type": "string"
    }
  },
  "$id": "https://test.json-schema.org/ref-dynamicAnchor-same-schema/root",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "$ref": "#items"
  },
  "type": "array"
}
"""
    root: list[Any]


from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Ref31Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "a": {
      "$id": "http://example.com/ref/absref/foobar.json",
      "type": "number"
    },
    "b": {
      "$id": "http://example.com/absref/foobar.json",
      "type": "string"
    }
  },
  "$id": "http://example.com/ref/absref.json",
  "$ref": "/absref/foobar.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""
    root: Any


from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Anchor2Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "A": {
      "$defs": {
        "B": {
          "$anchor": "foo",
          "type": "integer"
        }
      },
      "$id": "nested.json"
    }
  },
  "$id": "http://localhost:1234/draft2020-12/root",
  "$ref": "http://localhost:1234/draft2020-12/nested.json#foo",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""
    root: Any


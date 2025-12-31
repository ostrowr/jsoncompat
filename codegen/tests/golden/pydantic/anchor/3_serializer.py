from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Anchor3Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "A": {
      "$id": "child1",
      "allOf": [
        {
          "$anchor": "my_anchor",
          "$id": "child2",
          "type": "number"
        },
        {
          "$anchor": "my_anchor",
          "type": "string"
        }
      ]
    }
  },
  "$id": "http://localhost:1234/draft2020-12/foobar",
  "$ref": "child1#my_anchor",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""
    root: Any


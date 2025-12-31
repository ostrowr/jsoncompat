from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Ref29Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$ref": "http://example.com/ref/then",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "then": {
    "$id": "http://example.com/ref/then",
    "type": "integer"
  }
}
"""
    root: Any


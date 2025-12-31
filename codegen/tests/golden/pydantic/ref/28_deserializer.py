from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Ref28Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$ref": "http://example.com/ref/if",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "if": {
    "$id": "http://example.com/ref/if",
    "type": "integer"
  }
}
"""
    root: Any


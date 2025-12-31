from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = False

class Ref30Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$ref": "http://example.com/ref/else",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "$id": "http://example.com/ref/else",
    "type": "integer"
  }
}
"""
    root: Any


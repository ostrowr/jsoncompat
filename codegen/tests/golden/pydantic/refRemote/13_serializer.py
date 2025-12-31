from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = False

class Refremote13Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$ref": "http://localhost:1234/nested-absolute-ref-to-string.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""
    root: Any


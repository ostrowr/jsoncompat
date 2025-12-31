from typing import ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = False

class Unevaluatedproperties1Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "unevaluatedProperties": {
    "minLength": 3,
    "type": "string"
  }
}
"""
    model_config = ConfigDict(extra="allow")


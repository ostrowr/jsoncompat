from typing import Annotated, Any, ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = False

class Unevaluatedproperties32Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "x": {
      "$ref": "#"
    }
  },
  "type": "object",
  "unevaluatedProperties": false
}
"""
    model_config = ConfigDict(extra="allow")
    x: Annotated[Any | None, Field(default=None)]


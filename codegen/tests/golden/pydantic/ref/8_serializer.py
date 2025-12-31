from typing import Annotated

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Ref8Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "is-string": {
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "$ref": {
      "$ref": "#/$defs/is-string"
    }
  }
}
"""
    model_config = ConfigDict(extra="allow")
    ref: Annotated[str | None, Field(alias="$ref", default=None)]


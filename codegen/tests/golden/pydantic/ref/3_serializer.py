from typing import Annotated

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Ref3Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "percent%field": {
      "type": "integer"
    },
    "slash/field": {
      "type": "integer"
    },
    "tilde~field": {
      "type": "integer"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "percent": {
      "$ref": "#/$defs/percent%25field"
    },
    "slash": {
      "$ref": "#/$defs/slash~1field"
    },
    "tilde": {
      "$ref": "#/$defs/tilde~0field"
    }
  }
}
"""
    model_config = ConfigDict(extra="allow")
    percent: Annotated[int | float | None, Field(default=None)]
    slash: Annotated[int | float | None, Field(default=None)]
    tilde: Annotated[int | float | None, Field(default=None)]


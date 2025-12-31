from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Dynamicref18Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "false": false,
    "true": true
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "false": {
      "$dynamicRef": "#/$defs/false"
    },
    "true": {
      "$dynamicRef": "#/$defs/true"
    }
  }
}
"""
    model_config = ConfigDict(extra="allow")
    false: Annotated[Any | None, Field(default=None)]
    true: Annotated[Any | None, Field(default=None)]


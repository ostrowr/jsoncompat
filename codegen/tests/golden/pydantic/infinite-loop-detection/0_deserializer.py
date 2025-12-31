from typing import Annotated

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Infiniteloopdetection0Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "int": {
      "type": "integer"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "properties": {
        "foo": {
          "$ref": "#/$defs/int"
        }
      }
    },
    {
      "additionalProperties": {
        "$ref": "#/$defs/int"
      }
    }
  ]
}
"""
    model_config = ConfigDict(extra="allow")
    foo: Annotated[int | None, Field(default=None)]


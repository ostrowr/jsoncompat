from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Ref5Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "reffed": {
      "type": "array"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "$ref": "#/$defs/reffed",
      "maxItems": 2
    }
  }
}
"""
    model_config = ConfigDict(extra="allow")
    foo: Annotated[Any | None, Field(default=None)]


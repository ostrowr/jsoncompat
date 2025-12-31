from typing import Annotated

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Ref12Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "foo\"bar": {
      "type": "number"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo\"bar": {
      "$ref": "#/$defs/foo%22bar"
    }
  }
}
"""
    model_config = ConfigDict(extra="allow")
    foo_bar: Annotated[float | None, Field(alias="foo\"bar", default=None)]


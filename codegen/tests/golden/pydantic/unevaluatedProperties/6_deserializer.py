from typing import Annotated

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Unevaluatedproperties6Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "properties": {
        "bar": {
          "type": "string"
        }
      }
    }
  ],
  "properties": {
    "foo": {
      "type": "string"
    }
  },
  "type": "object",
  "unevaluatedProperties": false
}
"""
    model_config = ConfigDict(extra="allow")
    bar: Annotated[str | None, Field(default=None)]
    foo: Annotated[str | None, Field(default=None)]


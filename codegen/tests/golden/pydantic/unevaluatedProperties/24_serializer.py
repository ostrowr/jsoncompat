from typing import Annotated

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Unevaluatedproperties24Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "properties": {
        "foo": {
          "type": "string"
        }
      },
      "unevaluatedProperties": true
    }
  ],
  "type": "object",
  "unevaluatedProperties": false
}
"""
    model_config = ConfigDict(extra="allow")
    foo: Annotated[str | None, Field(default=None)]


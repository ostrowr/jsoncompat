from typing import Annotated

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Properties0Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar": {
      "type": "string"
    },
    "foo": {
      "type": "integer"
    }
  }
}
"""
    model_config = ConfigDict(extra="allow")
    bar: Annotated[str | None, Field(default=None)]
    foo: Annotated[int | float | None, Field(default=None)]


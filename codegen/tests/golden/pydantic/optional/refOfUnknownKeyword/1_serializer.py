from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Refofunknownkeyword1Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar": {
      "$ref": "#/properties/foo/unknown-keyword"
    },
    "foo": {
      "unknown-keyword": {
        "type": "integer"
      }
    }
  }
}
"""
    model_config = ConfigDict(extra="allow")
    bar: Annotated[int | float | None, Field(default=None)]
    foo: Annotated[Any | None, Field(default=None)]


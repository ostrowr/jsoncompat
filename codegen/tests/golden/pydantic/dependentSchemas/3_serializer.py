from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Dependentschemas3Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "dependentSchemas": {
    "foo": {
      "additionalProperties": false,
      "properties": {
        "bar": {}
      }
    }
  },
  "properties": {
    "foo": {}
  }
}
"""
    model_config = ConfigDict(extra="allow")
    foo: Annotated[Any | None, Field(default=None)]


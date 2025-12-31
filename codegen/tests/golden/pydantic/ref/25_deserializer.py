from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Ref25Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "bar": {
      "type": "string"
    }
  },
  "$id": "urn:uuid:deadbeef-1234-0000-0000-4321feebdaed",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "$ref": "urn:uuid:deadbeef-1234-0000-0000-4321feebdaed#/$defs/bar"
    }
  }
}
"""
    model_config = ConfigDict(extra="allow")
    foo: Annotated[Any | None, Field(default=None)]


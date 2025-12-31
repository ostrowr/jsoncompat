from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Ref26Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "bar": {
      "$anchor": "something",
      "type": "string"
    }
  },
  "$id": "urn:uuid:deadbeef-1234-ff00-00ff-4321feebdaed",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "$ref": "urn:uuid:deadbeef-1234-ff00-00ff-4321feebdaed#something"
    }
  }
}
"""
    model_config = ConfigDict(extra="allow")
    foo: Annotated[Any | None, Field(default=None)]


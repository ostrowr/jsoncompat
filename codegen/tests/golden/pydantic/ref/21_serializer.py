from typing import Annotated

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Ref21Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$comment": "URIs do not have to have HTTP(s) schemes",
  "$defs": {
    "bar": {
      "type": "string"
    }
  },
  "$id": "urn:uuid:deadbeef-1234-00ff-ff00-4321feebdaed",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "$ref": "#/$defs/bar"
    }
  }
}
"""
    model_config = ConfigDict(extra="allow")
    foo: Annotated[str | None, Field(default=None)]


from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Ref27Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "foo": {
      "$defs": {
        "bar": {
          "type": "string"
        }
      },
      "$id": "urn:uuid:deadbeef-4321-ffff-ffff-1234feebdaed",
      "$ref": "#/$defs/bar"
    }
  },
  "$ref": "urn:uuid:deadbeef-4321-ffff-ffff-1234feebdaed",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""
    root: Any


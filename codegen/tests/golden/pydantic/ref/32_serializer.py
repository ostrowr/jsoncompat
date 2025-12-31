from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Ref32Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "foo": {
      "type": "number"
    }
  },
  "$id": "file:///folder/file.json",
  "$ref": "#/$defs/foo",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""
    root: float


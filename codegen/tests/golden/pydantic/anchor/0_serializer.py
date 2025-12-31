from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Anchor0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "A": {
      "$anchor": "foo",
      "type": "integer"
    }
  },
  "$ref": "#foo",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""
    root: int | float


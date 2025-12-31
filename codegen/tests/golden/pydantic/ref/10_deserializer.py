from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, Impossible, SerializerBase, SerializerRootModel
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Ref10Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "bool": false
  },
  "$ref": "#/$defs/bool",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""
    root: Impossible


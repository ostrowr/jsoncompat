from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Additionalproperties6Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "type": "null"
  }
}
"""
    model_config = ConfigDict(extra="allow")


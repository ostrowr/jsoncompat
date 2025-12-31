from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Minproperties0Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "minProperties": 1
}
"""
    model_config = ConfigDict(extra="allow")


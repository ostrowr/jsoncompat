from typing import ClassVar

from json_schema_codegen_base import SerializerBase, DeserializerBase
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Ref10Deserializer(DeserializerBase):
    __json_schema__: ClassVar[str] = r"""
{
  "$defs": {
    "bool": false
  },
  "$ref": "#/$defs/bool",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""
    _validate_formats: ClassVar[bool] = _VALIDATE_FORMATS
    model_config = ConfigDict(extra="forbid")
    __json_compat_error__: ClassVar[str] = "unsupported schema feature at #: false schema"

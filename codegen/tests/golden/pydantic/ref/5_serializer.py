from typing import ClassVar

from json_schema_codegen_base import SerializerBase, DeserializerBase
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Ref5Serializer(SerializerBase):
    __json_schema__: ClassVar[str] = r"""
{
  "$defs": {
    "reffed": {
      "type": "array"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "$ref": "#/$defs/reffed",
      "maxItems": 2
    }
  }
}
"""
    _validate_formats: ClassVar[bool] = _VALIDATE_FORMATS
    model_config = ConfigDict(extra="forbid")
    __json_compat_error__: ClassVar[str] = "unsupported schema feature at #/properties/foo/allOf/0: allOf with non-object schema"

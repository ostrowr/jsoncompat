from typing import ClassVar

from json_schema_codegen_base import SerializerBase, DeserializerBase
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Ref28Deserializer(DeserializerBase):
    __json_schema__: ClassVar[str] = r"""
{
  "$ref": "http://example.com/ref/if",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "if": {
    "$id": "http://example.com/ref/if",
    "type": "integer"
  }
}
"""
    _validate_formats: ClassVar[bool] = _VALIDATE_FORMATS
    model_config = ConfigDict(extra="forbid")
    __json_compat_error__: ClassVar[str] = "unsupported schema feature at #/allOf/0: allOf with non-object schema"

from typing import ClassVar

from json_schema_codegen_base import SerializerBase, DeserializerBase
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class CrossDraft0Serializer(SerializerBase):
    __json_schema__: ClassVar[str] = r"""
{
  "$ref": "http://localhost:1234/draft2019-09/ignore-prefixItems.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "array"
}
"""
    _validate_formats: ClassVar[bool] = _VALIDATE_FORMATS
    model_config = ConfigDict(extra="forbid")
    __json_compat_error__: ClassVar[str] = "unsupported schema feature at #/allOf/0: allOf with non-object schema"

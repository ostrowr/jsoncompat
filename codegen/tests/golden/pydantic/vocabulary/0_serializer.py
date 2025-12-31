from typing import ClassVar

from json_schema_codegen_base import SerializerBase, DeserializerBase
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Vocabulary0Serializer(SerializerBase):
    __json_schema__: ClassVar[str] = r"""
{
  "$id": "https://schema/using/no/validation",
  "$schema": "http://localhost:1234/draft2020-12/metaschema-no-validation.json",
  "properties": {
    "badProperty": false,
    "numberProperty": {
      "minimum": 10
    }
  }
}
"""
    _validate_formats: ClassVar[bool] = _VALIDATE_FORMATS
    model_config = ConfigDict(extra="forbid")
    __json_compat_error__: ClassVar[str] = "unsupported schema feature at #/properties/badProperty: false schema"

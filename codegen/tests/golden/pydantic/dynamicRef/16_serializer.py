from typing import ClassVar

from json_schema_codegen_base import SerializerBase, DeserializerBase
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Dynamicref16Serializer(SerializerBase):
    __json_schema__: ClassVar[str] = r"""
{
  "$id": "http://localhost:1234/draft2020-12/strict-extendible-allof-ref-first.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "$defs": {
        "elements": {
          "$dynamicAnchor": "elements",
          "additionalProperties": false,
          "properties": {
            "a": true
          },
          "required": [
            "a"
          ]
        }
      }
    },
    {
      "$ref": "extendible-dynamic-ref.json"
    }
  ]
}
"""
    _validate_formats: ClassVar[bool] = _VALIDATE_FORMATS
    model_config = ConfigDict(extra="forbid")
    __json_compat_error__: ClassVar[str] = "unsupported schema feature at #/allOf/0: allOf with non-object schema"

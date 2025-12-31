from typing import ClassVar

from json_schema_codegen_base import SerializerBase, DeserializerBase
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Dynamicref15Serializer(SerializerBase):
    __json_schema__: ClassVar[str] = r"""
{
  "$id": "http://localhost:1234/draft2020-12/strict-extendible-allof-defs-first.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "$ref": "extendible-dynamic-ref.json"
    },
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
    }
  ]
}
"""
    _validate_formats: ClassVar[bool] = _VALIDATE_FORMATS
    model_config = ConfigDict(extra="forbid")
    __json_compat_error__: ClassVar[str] = "unsupported schema feature at #/allOf/0: allOf with non-object schema"

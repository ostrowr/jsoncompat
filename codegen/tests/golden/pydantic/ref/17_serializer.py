from typing import ClassVar

from json_schema_codegen_base import SerializerBase, DeserializerBase
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Ref17Serializer(SerializerBase):
    __json_schema__: ClassVar[str] = r"""
{
  "$defs": {
    "x": {
      "$id": "http://example.com/b/c.json",
      "not": {
        "$defs": {
          "y": {
            "$id": "d.json",
            "type": "number"
          }
        }
      }
    }
  },
  "$id": "http://example.com/a.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "$ref": "http://example.com/b/d.json"
    }
  ]
}
"""
    _validate_formats: ClassVar[bool] = _VALIDATE_FORMATS
    model_config = ConfigDict(extra="forbid")
    __json_compat_error__: ClassVar[str] = "unsupported schema feature at #/allOf/0: allOf with non-object schema"

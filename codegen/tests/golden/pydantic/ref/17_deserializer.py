from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = False

class Ref17Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
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
    root: Any


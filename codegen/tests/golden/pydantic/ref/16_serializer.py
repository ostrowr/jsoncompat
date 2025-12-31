from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_VALIDATE_FORMATS = False

class Ref16Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$id": "http://example.com/schema-refs-absolute-uris-defs1.json",
  "$ref": "schema-refs-absolute-uris-defs2.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "$defs": {
        "inner": {
          "properties": {
            "bar": {
              "type": "string"
            }
          }
        }
      },
      "$id": "http://example.com/schema-refs-absolute-uris-defs2.json",
      "$ref": "#/$defs/inner"
    }
  }
}
"""
    root: Any


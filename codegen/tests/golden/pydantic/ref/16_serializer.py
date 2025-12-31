from typing import ClassVar

from json_schema_codegen_base import SerializerBase, DeserializerBase
from pydantic import ConfigDict

_VALIDATE_FORMATS = False

class Ref16Serializer(SerializerBase):
    __json_schema__: ClassVar[str] = r"""
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
    _validate_formats: ClassVar[bool] = _VALIDATE_FORMATS
    model_config = ConfigDict(extra="forbid")
    __json_compat_error__: ClassVar[str] = "unsupported schema feature at #/allOf/0: allOf with non-object schema"

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, Impossible, SerializerBase
from pydantic import ConfigDict, Field

_VALIDATE_FORMATS = False

class Refremote6Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = r"""
{
  "$defs": {
    "baz": {
      "$defs": {
        "bar": {
          "items": {
            "$ref": "folderInteger.json"
          },
          "type": "array"
        }
      },
      "$id": "baseUriChangeFolderInSubschema/"
    }
  },
  "$id": "http://localhost:1234/draft2020-12/scope_change_defs2.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "list": {
      "$ref": "baseUriChangeFolderInSubschema/#/$defs/bar"
    }
  },
  "type": "object"
}
"""
    model_config = ConfigDict(extra="allow")
    list: Annotated[Any | None, Field(default=None)]


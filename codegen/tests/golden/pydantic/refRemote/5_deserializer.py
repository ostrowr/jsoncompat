"""
Schema:
{
  "$defs": {
    "baz": {
      "$id": "baseUriChangeFolder/",
      "items": {
        "$ref": "folderInteger.json"
      },
      "type": "array"
    }
  },
  "$id": "http://localhost:1234/draft2020-12/scope_change_defs1.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "list": {
      "$ref": "baseUriChangeFolder/"
    }
  },
  "type": "object"
}

Tests:
[
  {
    "data": {
      "list": [
        1
      ]
    },
    "description": "number is valid",
    "valid": true
  },
  {
    "data": {
      "list": [
        "a"
      ]
    },
    "description": "string is invalid",
    "valid": false
  }
]
"""

from typing import Annotated, Any, ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$defs": {
    "baz": {
      "$id": "baseUriChangeFolder/",
      "items": {
        "$ref": "folderInteger.json"
      },
      "type": "array"
    }
  },
  "$id": "http://localhost:1234/draft2020-12/scope_change_defs1.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "list": {
      "$ref": "baseUriChangeFolder/"
    }
  },
  "type": "object"
}
"""

_VALIDATE_FORMATS = False

class Refremote5Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    model_config = ConfigDict(extra="allow")
    list: Annotated[list[Any] | None, Field(default=None)]


"""
Schema:
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

_VALIDATE_FORMATS = False

class Refremote6Deserializer(DeserializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    model_config = ConfigDict(extra="allow")
    list: Annotated[Any | None, Field(default=None)]


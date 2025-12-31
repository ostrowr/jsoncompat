"""
Schema:
{
  "$id": "http://localhost:1234/draft2020-12/",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "$id": "baseUriChange/",
    "items": {
      "$ref": "folderInteger.json"
    }
  }
}

Tests:
[
  {
    "data": [
      [
        1
      ]
    ],
    "description": "base URI change ref valid",
    "valid": true
  },
  {
    "data": [
      [
        "a"
      ]
    ],
    "description": "base URI change ref invalid",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter, model_validator
from pydantic.functional_validators import BeforeValidator

_JSON_SCHEMA = r"""
{
  "$id": "http://localhost:1234/draft2020-12/",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "$id": "baseUriChange/",
    "items": {
      "$ref": "folderInteger.json"
    }
  }
}
"""

_VALIDATE_FORMATS = False

class Refremote4Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


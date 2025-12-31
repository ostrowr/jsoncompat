"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "prefixItems": [
    {
      "type": "integer"
    },
    {
      "$ref": "#/prefixItems/0"
    }
  ]
}

Tests:
[
  {
    "data": [
      1,
      2
    ],
    "description": "match array",
    "valid": true
  },
  {
    "data": [
      1,
      "foo"
    ],
    "description": "mismatch array",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "prefixItems": [
    {
      "type": "integer"
    },
    {
      "$ref": "#/prefixItems/0"
    }
  ]
}
"""

_VALIDATE_FORMATS = False

class Ref2Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


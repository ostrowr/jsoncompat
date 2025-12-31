"""
Schema:
{
  "$defs": {
    "A": {
      "$defs": {
        "B": {
          "$anchor": "foo",
          "type": "integer"
        }
      },
      "$id": "nested.json"
    }
  },
  "$id": "http://localhost:1234/draft2020-12/root",
  "$ref": "http://localhost:1234/draft2020-12/nested.json#foo",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": 1,
    "description": "match",
    "valid": true
  },
  {
    "data": "a",
    "description": "mismatch",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$defs": {
    "A": {
      "$defs": {
        "B": {
          "$anchor": "foo",
          "type": "integer"
        }
      },
      "$id": "nested.json"
    }
  },
  "$id": "http://localhost:1234/draft2020-12/root",
  "$ref": "http://localhost:1234/draft2020-12/nested.json#foo",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""

_VALIDATE_FORMATS = False

class Anchor2Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


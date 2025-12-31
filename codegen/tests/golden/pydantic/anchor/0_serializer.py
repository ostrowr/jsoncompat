"""
Schema:
{
  "$defs": {
    "A": {
      "$anchor": "foo",
      "type": "integer"
    }
  },
  "$ref": "#foo",
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

from typing import ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$defs": {
    "A": {
      "$anchor": "foo",
      "type": "integer"
    }
  },
  "$ref": "#foo",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}
"""

_VALIDATE_FORMATS = False

class Anchor0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: int | float


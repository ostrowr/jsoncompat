"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "propertyNames": {
    "pattern": "^a+$"
  }
}

Tests:
[
  {
    "data": {
      "a": {},
      "aa": {},
      "aaa": {}
    },
    "description": "matching property names valid",
    "valid": true
  },
  {
    "data": {
      "aaA": {}
    },
    "description": "non-matching property name is invalid",
    "valid": false
  },
  {
    "data": {},
    "description": "object without properties is valid",
    "valid": true
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "propertyNames": {
    "pattern": "^a+$"
  }
}
"""

_VALIDATE_FORMATS = False

class Propertynames1Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


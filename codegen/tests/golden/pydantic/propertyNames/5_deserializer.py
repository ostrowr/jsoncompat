"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "propertyNames": {
    "enum": [
      "foo",
      "bar"
    ]
  }
}

Tests:
[
  {
    "data": {
      "foo": 1
    },
    "description": "object with property foo is valid",
    "valid": true
  },
  {
    "data": {
      "bar": 1,
      "foo": 1
    },
    "description": "object with property foo and bar is valid",
    "valid": true
  },
  {
    "data": {
      "baz": 1
    },
    "description": "object with any other property is invalid",
    "valid": false
  },
  {
    "data": {},
    "description": "empty object is valid",
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
    "enum": [
      "foo",
      "bar"
    ]
  }
}
"""

_VALIDATE_FORMATS = False

class Propertynames5Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


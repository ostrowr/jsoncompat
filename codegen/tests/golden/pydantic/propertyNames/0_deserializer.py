"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "propertyNames": {
    "maxLength": 3
  }
}

Tests:
[
  {
    "data": {
      "f": {},
      "foo": {}
    },
    "description": "all property names valid",
    "valid": true
  },
  {
    "data": {
      "foo": {},
      "foobar": {}
    },
    "description": "some property names invalid",
    "valid": false
  },
  {
    "data": {},
    "description": "object without properties is valid",
    "valid": true
  },
  {
    "data": [
      1,
      2,
      3,
      4
    ],
    "description": "ignores arrays",
    "valid": true
  },
  {
    "data": "foobar",
    "description": "ignores strings",
    "valid": true
  },
  {
    "data": 12,
    "description": "ignores other non-objects",
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
    "maxLength": 3
  }
}
"""

_VALIDATE_FORMATS = False

class Propertynames0Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


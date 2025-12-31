"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "dependentSchemas": {
    "bar": false,
    "foo": true
  }
}

Tests:
[
  {
    "data": {
      "foo": 1
    },
    "description": "object with property having schema true is valid",
    "valid": true
  },
  {
    "data": {
      "bar": 2
    },
    "description": "object with property having schema false is invalid",
    "valid": false
  },
  {
    "data": {
      "bar": 2,
      "foo": 1
    },
    "description": "object with both properties is invalid",
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
  "dependentSchemas": {
    "bar": false,
    "foo": true
  }
}
"""

_VALIDATE_FORMATS = False

class Dependentschemas1Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "patternProperties": {
    "b.*": false,
    "f.*": true
  }
}

Tests:
[
  {
    "data": {
      "foo": 1
    },
    "description": "object with property matching schema true is valid",
    "valid": true
  },
  {
    "data": {
      "bar": 2
    },
    "description": "object with property matching schema false is invalid",
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
    "data": {
      "foobar": 1
    },
    "description": "object with a property matching both true and false is invalid",
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
  "patternProperties": {
    "b.*": false,
    "f.*": true
  }
}
"""

_VALIDATE_FORMATS = False

class Patternproperties3Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


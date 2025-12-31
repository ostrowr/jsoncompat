"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    6,
    "foo",
    [],
    true,
    {
      "foo": 12
    }
  ]
}

Tests:
[
  {
    "data": [],
    "description": "one of the enum is valid",
    "valid": true
  },
  {
    "data": null,
    "description": "something else is invalid",
    "valid": false
  },
  {
    "data": {
      "foo": false
    },
    "description": "objects are deep compared",
    "valid": false
  },
  {
    "data": {
      "foo": 12
    },
    "description": "valid object matches",
    "valid": true
  },
  {
    "data": {
      "boo": 42,
      "foo": 12
    },
    "description": "extra properties in object is invalid",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field, model_validator
from pydantic.functional_validators import BeforeValidator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    6,
    "foo",
    [],
    true,
    {
      "foo": 12
    }
  ]
}
"""

_VALIDATE_FORMATS = False

class Enum1Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


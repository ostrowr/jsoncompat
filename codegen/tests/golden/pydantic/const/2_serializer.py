"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": [
    {
      "foo": "bar"
    }
  ]
}

Tests:
[
  {
    "data": [
      {
        "foo": "bar"
      }
    ],
    "description": "same array is valid",
    "valid": true
  },
  {
    "data": [
      2
    ],
    "description": "another array item is invalid",
    "valid": false
  },
  {
    "data": [
      1,
      2,
      3
    ],
    "description": "array with additional items is invalid",
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
  "const": [
    {
      "foo": "bar"
    }
  ]
}
"""

_VALIDATE_FORMATS = False

class Const2Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


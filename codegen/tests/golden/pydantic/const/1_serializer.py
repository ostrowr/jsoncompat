"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": {
    "baz": "bax",
    "foo": "bar"
  }
}

Tests:
[
  {
    "data": {
      "baz": "bax",
      "foo": "bar"
    },
    "description": "same object is valid",
    "valid": true
  },
  {
    "data": {
      "baz": "bax",
      "foo": "bar"
    },
    "description": "same object with different property order is valid",
    "valid": true
  },
  {
    "data": {
      "foo": "bar"
    },
    "description": "another object is invalid",
    "valid": false
  },
  {
    "data": [
      1,
      2
    ],
    "description": "another type is invalid",
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
  "const": {
    "baz": "bax",
    "foo": "bar"
  }
}
"""

_VALIDATE_FORMATS = False

class Const1Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


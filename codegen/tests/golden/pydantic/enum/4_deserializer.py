"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    "foo\nbar",
    "foo\rbar"
  ]
}

Tests:
[
  {
    "data": "foo\nbar",
    "description": "member 1 is valid",
    "valid": true
  },
  {
    "data": "foo\rbar",
    "description": "member 2 is valid",
    "valid": true
  },
  {
    "data": "abc",
    "description": "another string is invalid",
    "valid": false
  }
]
"""

from typing import ClassVar, Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field, model_validator
from pydantic.functional_validators import BeforeValidator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    "foo\nbar",
    "foo\rbar"
  ]
}
"""

_VALIDATE_FORMATS = False

class Enum4Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Literal["foo\nbar", "foo\rbar"]


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "maxLength": 2
}

Tests:
[
  {
    "data": "f",
    "description": "shorter is valid",
    "valid": true
  },
  {
    "data": "fo",
    "description": "exact length is valid",
    "valid": true
  },
  {
    "data": "foo",
    "description": "too long is invalid",
    "valid": false
  },
  {
    "data": 100,
    "description": "ignores non-strings",
    "valid": true
  },
  {
    "data": "💩💩",
    "description": "two graphemes is long enough",
    "valid": true
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, TypeAdapter, model_validator
from pydantic.functional_validators import BeforeValidator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "maxLength": 2
}
"""

_VALIDATE_FORMATS = False

class Maxlength0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


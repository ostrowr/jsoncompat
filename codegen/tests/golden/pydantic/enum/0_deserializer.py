"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    1,
    2,
    3
  ]
}

Tests:
[
  {
    "data": 1,
    "description": "one of the enum is valid",
    "valid": true
  },
  {
    "data": 4,
    "description": "something else is invalid",
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
    1,
    2,
    3
  ]
}
"""

_VALIDATE_FORMATS = False

class Enum0Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Literal[1, 2, 3]


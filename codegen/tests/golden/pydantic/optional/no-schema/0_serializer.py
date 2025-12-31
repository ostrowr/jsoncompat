"""
Schema:
{
  "minLength": 2
}

Tests:
[
  {
    "data": "foo",
    "description": "a 3-character string is valid",
    "valid": true
  },
  {
    "data": "a",
    "description": "a 1-character string is not valid",
    "valid": false
  },
  {
    "data": 5,
    "description": "a non-string is valid",
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
  "minLength": 2
}
"""

_VALIDATE_FORMATS = False

class Noschema0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


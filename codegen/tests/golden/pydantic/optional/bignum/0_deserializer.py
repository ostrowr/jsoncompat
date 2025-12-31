"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "integer"
}

Tests:
[
  {
    "data": 1.2345678910111214e52,
    "description": "a bignum is an integer",
    "valid": true
  },
  {
    "data": -1.2345678910111214e52,
    "description": "a negative bignum is an integer",
    "valid": true
  }
]
"""

from typing import ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "integer"
}
"""

_VALIDATE_FORMATS = False

class Bignum0Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: int


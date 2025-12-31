"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "multipleOf": 0.5,
  "type": "integer"
}

Tests:
[
  {
    "data": 1e308,
    "description": "valid if optional overflow handling is implemented",
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
  "multipleOf": 0.5,
  "type": "integer"
}
"""

_VALIDATE_FORMATS = False

class Floatoverflow0Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: int


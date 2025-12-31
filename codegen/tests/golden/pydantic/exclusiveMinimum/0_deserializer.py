"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "exclusiveMinimum": 1.1
}

Tests:
[
  {
    "data": 1.2,
    "description": "above the exclusiveMinimum is valid",
    "valid": true
  },
  {
    "data": 1.1,
    "description": "boundary point is invalid",
    "valid": false
  },
  {
    "data": 0.6,
    "description": "below the exclusiveMinimum is invalid",
    "valid": false
  },
  {
    "data": "x",
    "description": "ignores non-numbers",
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
  "exclusiveMinimum": 1.1
}
"""

_VALIDATE_FORMATS = False

class Exclusiveminimum0Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


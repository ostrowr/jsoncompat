"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "maximum": 300
}

Tests:
[
  {
    "data": 299.97,
    "description": "below the maximum is invalid",
    "valid": true
  },
  {
    "data": 300,
    "description": "boundary point integer is valid",
    "valid": true
  },
  {
    "data": 300.0,
    "description": "boundary point float is valid",
    "valid": true
  },
  {
    "data": 300.5,
    "description": "above the maximum is invalid",
    "valid": false
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
  "maximum": 300
}
"""

_VALIDATE_FORMATS = False

class Maximum1Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "multipleOf": 1.5
}

Tests:
[
  {
    "data": 0,
    "description": "zero is multiple of anything",
    "valid": true
  },
  {
    "data": 4.5,
    "description": "4.5 is multiple of 1.5",
    "valid": true
  },
  {
    "data": 35,
    "description": "35 is not multiple of 1.5",
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
  "multipleOf": 1.5
}
"""

_VALIDATE_FORMATS = False

class Multipleof1Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "multipleOf": 0.0001
}

Tests:
[
  {
    "data": 0.0075,
    "description": "0.0075 is multiple of 0.0001",
    "valid": true
  },
  {
    "data": 0.00751,
    "description": "0.00751 is not multiple of 0.0001",
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
  "multipleOf": 0.0001
}
"""

_VALIDATE_FORMATS = False

class Multipleof2Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


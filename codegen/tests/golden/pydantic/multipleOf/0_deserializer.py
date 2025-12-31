"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "multipleOf": 2
}

Tests:
[
  {
    "data": 10,
    "description": "int by int",
    "valid": true
  },
  {
    "data": 7,
    "description": "int by int fail",
    "valid": false
  },
  {
    "data": "foo",
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
  "multipleOf": 2
}
"""

_VALIDATE_FORMATS = False

class Multipleof0Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


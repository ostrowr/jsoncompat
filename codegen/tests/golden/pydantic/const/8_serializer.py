"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": {
    "a": false
  }
}

Tests:
[
  {
    "data": {
      "a": false
    },
    "description": "{\"a\": false} is valid",
    "valid": true
  },
  {
    "data": {
      "a": 0
    },
    "description": "{\"a\": 0} is invalid",
    "valid": false
  },
  {
    "data": {
      "a": 0.0
    },
    "description": "{\"a\": 0.0} is invalid",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field, model_validator
from pydantic.functional_validators import BeforeValidator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": {
    "a": false
  }
}
"""

_VALIDATE_FORMATS = False

class Const8Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


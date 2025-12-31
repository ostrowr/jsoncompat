"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "then": {
    "const": 0
  }
}

Tests:
[
  {
    "data": 0,
    "description": "valid when valid against lone then",
    "valid": true
  },
  {
    "data": "hello",
    "description": "valid when invalid against lone then",
    "valid": true
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "then": {
    "const": 0
  }
}
"""

_VALIDATE_FORMATS = False

class Ifthenelse1Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


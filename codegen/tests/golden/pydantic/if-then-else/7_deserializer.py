"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "const": "else"
  },
  "if": true,
  "then": {
    "const": "then"
  }
}

Tests:
[
  {
    "data": "then",
    "description": "boolean schema true in if always chooses the then path (valid)",
    "valid": true
  },
  {
    "data": "else",
    "description": "boolean schema true in if always chooses the then path (invalid)",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "const": "else"
  },
  "if": true,
  "then": {
    "const": "then"
  }
}
"""

_VALIDATE_FORMATS = False

class Ifthenelse7Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


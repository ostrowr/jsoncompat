"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "const": "else"
  },
  "if": false,
  "then": {
    "const": "then"
  }
}

Tests:
[
  {
    "data": "then",
    "description": "boolean schema false in if always chooses the else path (invalid)",
    "valid": false
  },
  {
    "data": "else",
    "description": "boolean schema false in if always chooses the else path (valid)",
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
  "else": {
    "const": "else"
  },
  "if": false,
  "then": {
    "const": "then"
  }
}
"""

_VALIDATE_FORMATS = False

class Ifthenelse8Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "const": "other"
  },
  "if": {
    "maxLength": 4
  },
  "then": {
    "const": "yes"
  }
}

Tests:
[
  {
    "data": "yes",
    "description": "yes redirects to then and passes",
    "valid": true
  },
  {
    "data": "other",
    "description": "other redirects to else and passes",
    "valid": true
  },
  {
    "data": "no",
    "description": "no redirects to then and fails",
    "valid": false
  },
  {
    "data": "invalid",
    "description": "invalid redirects to else and fails",
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
    "const": "other"
  },
  "if": {
    "maxLength": 4
  },
  "then": {
    "const": "yes"
  }
}
"""

_VALIDATE_FORMATS = False

class Ifthenelse9Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


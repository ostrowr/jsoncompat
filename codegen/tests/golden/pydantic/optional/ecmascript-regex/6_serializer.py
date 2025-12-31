"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\w$",
  "type": "string"
}

Tests:
[
  {
    "data": "a",
    "description": "ASCII 'a' matches",
    "valid": true
  },
  {
    "data": "é",
    "description": "latin-1 e-acute does not match (unlike e.g. Python)",
    "valid": false
  }
]
"""

from typing import ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\w$",
  "type": "string"
}
"""

_VALIDATE_FORMATS = False

class Ecmascriptregex6Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: str


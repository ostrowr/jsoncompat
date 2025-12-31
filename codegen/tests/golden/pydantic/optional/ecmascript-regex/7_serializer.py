"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\W$",
  "type": "string"
}

Tests:
[
  {
    "data": "a",
    "description": "ASCII 'a' does not match",
    "valid": false
  },
  {
    "data": "é",
    "description": "latin-1 e-acute matches (unlike e.g. Python)",
    "valid": true
  }
]
"""

from typing import ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\W$",
  "type": "string"
}
"""

_VALIDATE_FORMATS = False

class Ecmascriptregex7Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: str


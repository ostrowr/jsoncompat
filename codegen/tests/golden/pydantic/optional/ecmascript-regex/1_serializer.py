"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\t$",
  "type": "string"
}

Tests:
[
  {
    "data": "\\t",
    "description": "does not match",
    "valid": false
  },
  {
    "data": "\t",
    "description": "matches",
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
  "pattern": "^\\t$",
  "type": "string"
}
"""

_VALIDATE_FORMATS = False

class Ecmascriptregex1Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: str


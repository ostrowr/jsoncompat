"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\cC$",
  "type": "string"
}

Tests:
[
  {
    "data": "\\cC",
    "description": "does not match",
    "valid": false
  },
  {
    "data": "\u0003",
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
  "pattern": "^\\cC$",
  "type": "string"
}
"""

_VALIDATE_FORMATS = False

class Ecmascriptregex2Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: str


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\D$",
  "type": "string"
}

Tests:
[
  {
    "data": "0",
    "description": "ASCII zero does not match",
    "valid": false
  },
  {
    "data": "߀",
    "description": "NKO DIGIT ZERO matches (unlike e.g. Python)",
    "valid": true
  },
  {
    "data": "߀",
    "description": "NKO DIGIT ZERO (as \\u escape) matches",
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
  "pattern": "^\\D$",
  "type": "string"
}
"""

_VALIDATE_FORMATS = False

class Ecmascriptregex5Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: str


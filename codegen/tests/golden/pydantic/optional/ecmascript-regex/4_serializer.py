"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^\\d$",
  "type": "string"
}

Tests:
[
  {
    "data": "0",
    "description": "ASCII zero matches",
    "valid": true
  },
  {
    "data": "߀",
    "description": "NKO DIGIT ZERO does not match (unlike e.g. Python)",
    "valid": false
  },
  {
    "data": "߀",
    "description": "NKO DIGIT ZERO (as \\u escape) does not match",
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
  "pattern": "^\\d$",
  "type": "string"
}
"""

_VALIDATE_FORMATS = False

class Ecmascriptregex4Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: str


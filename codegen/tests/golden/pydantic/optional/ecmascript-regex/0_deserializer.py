"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^abc$",
  "type": "string"
}

Tests:
[
  {
    "data": "abc\\n",
    "description": "matches in Python, but not in ECMA 262",
    "valid": false
  },
  {
    "data": "abc",
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
  "pattern": "^abc$",
  "type": "string"
}
"""

_VALIDATE_FORMATS = False

class Ecmascriptregex0Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: str


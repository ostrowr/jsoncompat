"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "unevaluatedProperties": true
}

Tests:
[
  {
    "data": {},
    "description": "with no unevaluated properties",
    "valid": true
  },
  {
    "data": {
      "foo": "foo"
    },
    "description": "with unevaluated properties",
    "valid": true
  }
]
"""

from typing import ClassVar

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "unevaluatedProperties": true
}
"""

_VALIDATE_FORMATS = False

class Unevaluatedproperties0Serializer(SerializerBase):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    model_config = ConfigDict(extra="allow")


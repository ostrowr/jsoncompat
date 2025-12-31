"""
Schema:
{
  "$defs": {
    "a_string": {
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    {
      "$ref": "#/$defs/a_string"
    }
  ]
}

Tests:
[
  {
    "data": "this is a string",
    "description": "do not evaluate the $ref inside the enum, matching any string",
    "valid": false
  },
  {
    "data": {
      "type": "string"
    },
    "description": "do not evaluate the $ref inside the enum, definition exact match",
    "valid": false
  },
  {
    "data": {
      "$ref": "#/$defs/a_string"
    },
    "description": "match the enum exactly",
    "valid": true
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field, model_validator
from pydantic.functional_validators import BeforeValidator

_JSON_SCHEMA = r"""
{
  "$defs": {
    "a_string": {
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    {
      "$ref": "#/$defs/a_string"
    }
  ]
}
"""

_VALIDATE_FORMATS = False

class Ref14Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


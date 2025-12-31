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

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

class Ref14Serializer(SerializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _allowed=[{"$ref": "#/$defs/a_string"}]: _validate_literal(v, _allowed))]


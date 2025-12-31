"""
Schema:
{
  "$defs": {
    "anchor_in_enum": {
      "enum": [
        {
          "$anchor": "my_anchor",
          "type": "null"
        }
      ]
    },
    "real_identifier_in_schema": {
      "$anchor": "my_anchor",
      "type": "string"
    },
    "zzz_anchor_in_const": {
      "const": {
        "$anchor": "my_anchor",
        "type": "null"
      }
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "$ref": "#/$defs/anchor_in_enum"
    },
    {
      "$ref": "#my_anchor"
    }
  ]
}

Tests:
[
  {
    "data": {
      "$anchor": "my_anchor",
      "type": "null"
    },
    "description": "exact match to enum, and type matches",
    "valid": true
  },
  {
    "data": {
      "type": "null"
    },
    "description": "in implementations that strip $anchor, this may match either $def",
    "valid": false
  },
  {
    "data": "a string to match #/$defs/anchor_in_enum",
    "description": "match $ref to $anchor",
    "valid": true
  },
  {
    "data": 1,
    "description": "no match on enum or $ref to $anchor",
    "valid": false
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
    "anchor_in_enum": {
      "enum": [
        {
          "$anchor": "my_anchor",
          "type": "null"
        }
      ]
    },
    "real_identifier_in_schema": {
      "$anchor": "my_anchor",
      "type": "string"
    },
    "zzz_anchor_in_const": {
      "const": {
        "$anchor": "my_anchor",
        "type": "null"
      }
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "$ref": "#/$defs/anchor_in_enum"
    },
    {
      "$ref": "#my_anchor"
    }
  ]
}
"""

_VALIDATE_FORMATS = False

class Anchor0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any | None


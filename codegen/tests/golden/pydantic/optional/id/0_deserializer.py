"""
Schema:
{
  "$defs": {
    "id_in_enum": {
      "enum": [
        {
          "$id": "https://localhost:1234/draft2020-12/id/my_identifier.json",
          "type": "null"
        }
      ]
    },
    "real_id_in_schema": {
      "$id": "https://localhost:1234/draft2020-12/id/my_identifier.json",
      "type": "string"
    },
    "zzz_id_in_const": {
      "const": {
        "$id": "https://localhost:1234/draft2020-12/id/my_identifier.json",
        "type": "null"
      }
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "$ref": "#/$defs/id_in_enum"
    },
    {
      "$ref": "https://localhost:1234/draft2020-12/id/my_identifier.json"
    }
  ]
}

Tests:
[
  {
    "data": {
      "$id": "https://localhost:1234/draft2020-12/id/my_identifier.json",
      "type": "null"
    },
    "description": "exact match to enum, and type matches",
    "valid": true
  },
  {
    "data": "a string to match #/$defs/id_in_enum",
    "description": "match $ref to $id",
    "valid": true
  },
  {
    "data": 1,
    "description": "no match on enum or $ref to $id",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel, _validate_literal
from pydantic import ConfigDict, Field
from pydantic.functional_validators import BeforeValidator

class Id0Deserializer(DeserializerRootModel):
    root: Annotated[Any, BeforeValidator(lambda v, _allowed=[{"$id": "https://localhost:1234/draft2020-12/id/my_identifier.json", "type": "null"}]: _validate_literal(v, _allowed))] | str


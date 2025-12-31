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

from pydantic import BaseModel, ConfigDict

class Id0Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported enum/const value at #/anyOf/0: {\"$id\":\"https://localhost:1234/draft2020-12/id/my_identifier.json\",\"type\":\"null\"}")

"""
Schema:
{
  "$id": "http://localhost:1234/draft2020-12/strict-extendible-allof-defs-first.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "$ref": "extendible-dynamic-ref.json"
    },
    {
      "$defs": {
        "elements": {
          "$dynamicAnchor": "elements",
          "additionalProperties": false,
          "properties": {
            "a": true
          },
          "required": [
            "a"
          ]
        }
      }
    }
  ]
}

Tests:
[
  {
    "data": {
      "a": true
    },
    "description": "incorrect parent schema",
    "valid": false
  },
  {
    "data": {
      "elements": [
        {
          "b": 1
        }
      ]
    },
    "description": "incorrect extended schema",
    "valid": false
  },
  {
    "data": {
      "elements": [
        {
          "a": 1
        }
      ]
    },
    "description": "correct extended schema",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Dynamicref15Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")

"""
Schema:
{
  "$defs": {
    "x": {
      "$id": "http://example.com/b/c.json",
      "not": {
        "$defs": {
          "y": {
            "$id": "d.json",
            "type": "number"
          }
        }
      }
    }
  },
  "$id": "http://example.com/a.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "$ref": "http://example.com/b/d.json"
    }
  ]
}

Tests:
[
  {
    "data": 1,
    "description": "number is valid",
    "valid": true
  },
  {
    "data": "a",
    "description": "non-number is invalid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Ref17Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")

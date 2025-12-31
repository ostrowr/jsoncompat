"""
Schema:
{
  "$defs": {
    "": {
      "$defs": {
        "": {
          "type": "number"
        }
      }
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "$ref": "#/$defs//$defs/"
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

class Ref34Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")

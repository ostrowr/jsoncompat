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

from pydantic import BaseModel, ConfigDict

class Ref14Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported enum/const value at #: {\"$ref\":\"#/$defs/a_string\"}")

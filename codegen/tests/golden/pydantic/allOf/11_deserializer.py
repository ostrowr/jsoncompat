"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "multipleOf": 2
    }
  ],
  "anyOf": [
    {
      "multipleOf": 3
    }
  ],
  "oneOf": [
    {
      "multipleOf": 5
    }
  ]
}

Tests:
[
  {
    "data": 1,
    "description": "allOf: false, anyOf: false, oneOf: false",
    "valid": false
  },
  {
    "data": 5,
    "description": "allOf: false, anyOf: false, oneOf: true",
    "valid": false
  },
  {
    "data": 3,
    "description": "allOf: false, anyOf: true, oneOf: false",
    "valid": false
  },
  {
    "data": 15,
    "description": "allOf: false, anyOf: true, oneOf: true",
    "valid": false
  },
  {
    "data": 2,
    "description": "allOf: true, anyOf: false, oneOf: false",
    "valid": false
  },
  {
    "data": 10,
    "description": "allOf: true, anyOf: false, oneOf: true",
    "valid": false
  },
  {
    "data": 6,
    "description": "allOf: true, anyOf: true, oneOf: false",
    "valid": false
  },
  {
    "data": 30,
    "description": "allOf: true, anyOf: true, oneOf: true",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Allof11Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")

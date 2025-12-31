"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "properties": {
        "foo": true
      },
      "unevaluatedProperties": false
    }
  ],
  "anyOf": [
    {
      "properties": {
        "bar": true
      }
    }
  ],
  "type": "object"
}

Tests:
[
  {
    "data": {
      "bar": 1,
      "foo": 1
    },
    "description": "base case: both properties present",
    "valid": false
  },
  {
    "data": {
      "foo": 1
    },
    "description": "in place applicator siblings, bar is missing",
    "valid": true
  },
  {
    "data": {
      "bar": 1
    },
    "description": "in place applicator siblings, foo is missing",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluatedproperties30Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")

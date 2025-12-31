"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "contains": {
        "multipleOf": 2
      }
    },
    {
      "contains": {
        "multipleOf": 3
      }
    }
  ],
  "unevaluatedItems": {
    "multipleOf": 5
  }
}

Tests:
[
  {
    "data": [
      2,
      3,
      4,
      5,
      6
    ],
    "description": "5 not evaluated, passes unevaluatedItems",
    "valid": true
  },
  {
    "data": [
      2,
      3,
      4,
      7,
      8
    ],
    "description": "7 not evaluated, fails unevaluatedItems",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluateditems22Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")

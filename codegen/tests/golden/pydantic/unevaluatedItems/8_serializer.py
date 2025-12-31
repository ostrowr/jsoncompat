"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "items": {
        "type": "string"
      }
    },
    true
  ],
  "unevaluatedItems": {
    "type": "boolean"
  }
}

Tests:
[
  {
    "data": [
      true,
      false
    ],
    "description": "with only (valid) additional items",
    "valid": true
  },
  {
    "data": [
      "yes",
      "no"
    ],
    "description": "with no additional items",
    "valid": true
  },
  {
    "data": [
      "yes",
      false
    ],
    "description": "with invalid additional item",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluateditems8Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")

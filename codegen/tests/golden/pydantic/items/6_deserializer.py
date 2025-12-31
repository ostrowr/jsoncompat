"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "prefixItems": [
        {
          "minimum": 3
        }
      ]
    }
  ],
  "items": {
    "minimum": 5
  }
}

Tests:
[
  {
    "data": [
      3,
      5
    ],
    "description": "prefixItems in allOf does not constrain items, invalid case",
    "valid": false
  },
  {
    "data": [
      5,
      5
    ],
    "description": "prefixItems in allOf does not constrain items, valid case",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Items6Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")

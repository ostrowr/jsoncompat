"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "type": "integer"
  },
  "prefixItems": [
    {
      "type": "string"
    }
  ]
}

Tests:
[
  {
    "data": [
      "x",
      2,
      3
    ],
    "description": "valid items",
    "valid": true
  },
  {
    "data": [
      "x",
      "y"
    ],
    "description": "wrong type of second item",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Items7Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

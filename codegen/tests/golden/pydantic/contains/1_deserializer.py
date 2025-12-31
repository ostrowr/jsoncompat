"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contains": {
    "const": 5
  }
}

Tests:
[
  {
    "data": [
      3,
      4,
      5
    ],
    "description": "array with item 5 is valid",
    "valid": true
  },
  {
    "data": [
      3,
      4,
      5,
      5
    ],
    "description": "array with two items 5 is valid",
    "valid": true
  },
  {
    "data": [
      1,
      2,
      3,
      4
    ],
    "description": "array without item 5 is invalid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Contains1Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

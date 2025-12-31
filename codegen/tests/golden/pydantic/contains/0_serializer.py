"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contains": {
    "minimum": 5
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
    "description": "array with item matching schema (5) is valid",
    "valid": true
  },
  {
    "data": [
      3,
      4,
      6
    ],
    "description": "array with item matching schema (6) is valid",
    "valid": true
  },
  {
    "data": [
      3,
      4,
      5,
      6
    ],
    "description": "array with two items matching schema (5, 6) is valid",
    "valid": true
  },
  {
    "data": [
      2,
      3,
      4
    ],
    "description": "array without items matching schema is invalid",
    "valid": false
  },
  {
    "data": [],
    "description": "empty array is invalid",
    "valid": false
  },
  {
    "data": {},
    "description": "not array is valid",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Contains0Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

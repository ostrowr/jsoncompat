"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": false,
  "prefixItems": [
    {},
    {},
    {}
  ]
}

Tests:
[
  {
    "data": [],
    "description": "empty array",
    "valid": true
  },
  {
    "data": [
      1
    ],
    "description": "fewer number of items present (1)",
    "valid": true
  },
  {
    "data": [
      1,
      2
    ],
    "description": "fewer number of items present (2)",
    "valid": true
  },
  {
    "data": [
      1,
      2,
      3
    ],
    "description": "equal number of items present",
    "valid": true
  },
  {
    "data": [
      1,
      2,
      3,
      4
    ],
    "description": "additional items are not permitted",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Items5Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

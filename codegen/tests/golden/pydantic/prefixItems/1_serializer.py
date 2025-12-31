"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "prefixItems": [
    true,
    false
  ]
}

Tests:
[
  {
    "data": [
      1
    ],
    "description": "array with one item is valid",
    "valid": true
  },
  {
    "data": [
      1,
      "foo"
    ],
    "description": "array with two items is invalid",
    "valid": false
  },
  {
    "data": [],
    "description": "empty array is valid",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Prefixitems1Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contains": {
    "const": 1
  },
  "minContains": 2
}

Tests:
[
  {
    "data": [],
    "description": "empty data",
    "valid": false
  },
  {
    "data": [
      1
    ],
    "description": "all elements match, invalid minContains",
    "valid": false
  },
  {
    "data": [
      1,
      2
    ],
    "description": "some elements match, invalid minContains",
    "valid": false
  },
  {
    "data": [
      1,
      1
    ],
    "description": "all elements match, valid minContains (exactly as needed)",
    "valid": true
  },
  {
    "data": [
      1,
      1,
      1
    ],
    "description": "all elements match, valid minContains (more than needed)",
    "valid": true
  },
  {
    "data": [
      1,
      2,
      1
    ],
    "description": "some elements match, valid minContains",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Mincontains2Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

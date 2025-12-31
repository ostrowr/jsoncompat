"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contains": {
    "const": 1
  },
  "minContains": 1
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
      2
    ],
    "description": "no elements match",
    "valid": false
  },
  {
    "data": [
      1
    ],
    "description": "single element matches, valid minContains",
    "valid": true
  },
  {
    "data": [
      1,
      2
    ],
    "description": "some elements match, valid minContains",
    "valid": true
  },
  {
    "data": [
      1,
      1
    ],
    "description": "all elements match, valid minContains",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Mincontains1Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contains": {
    "const": 1
  },
  "maxContains": 1
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
    "description": "all elements match, valid maxContains",
    "valid": true
  },
  {
    "data": [
      1,
      1
    ],
    "description": "all elements match, invalid maxContains",
    "valid": false
  },
  {
    "data": [
      1,
      2
    ],
    "description": "some elements match, valid maxContains",
    "valid": true
  },
  {
    "data": [
      1,
      2,
      1
    ],
    "description": "some elements match, invalid maxContains",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Maxcontains1Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contains": {
    "const": 1
  },
  "maxContains": 1,
  "minContains": 0
}

Tests:
[
  {
    "data": [],
    "description": "empty data",
    "valid": true
  },
  {
    "data": [
      1
    ],
    "description": "not more than maxContains",
    "valid": true
  },
  {
    "data": [
      1,
      1
    ],
    "description": "too many",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Mincontains7Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contains": {
    "const": 1
  },
  "maxContains": 1,
  "minContains": 3
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
    "description": "invalid minContains",
    "valid": false
  },
  {
    "data": [
      1,
      1,
      1
    ],
    "description": "invalid maxContains",
    "valid": false
  },
  {
    "data": [
      1,
      1
    ],
    "description": "invalid maxContains and minContains",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Mincontains5Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

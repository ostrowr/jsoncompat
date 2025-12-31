"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contains": {
    "const": 1
  },
  "maxContains": 3,
  "minContains": 1
}

Tests:
[
  {
    "data": [],
    "description": "actual < minContains < maxContains",
    "valid": false
  },
  {
    "data": [
      1,
      1
    ],
    "description": "minContains < actual < maxContains",
    "valid": true
  },
  {
    "data": [
      1,
      1,
      1,
      1
    ],
    "description": "minContains < maxContains < actual",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Maxcontains3Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

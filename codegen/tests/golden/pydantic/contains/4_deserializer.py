"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contains": {
    "multipleOf": 3
  },
  "items": {
    "multipleOf": 2
  }
}

Tests:
[
  {
    "data": [
      2,
      4,
      8
    ],
    "description": "matches items, does not match contains",
    "valid": false
  },
  {
    "data": [
      3,
      6,
      9
    ],
    "description": "does not match items, matches contains",
    "valid": false
  },
  {
    "data": [
      6,
      12
    ],
    "description": "matches both items and contains",
    "valid": true
  },
  {
    "data": [
      1,
      5
    ],
    "description": "matches neither items nor contains",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Contains4Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

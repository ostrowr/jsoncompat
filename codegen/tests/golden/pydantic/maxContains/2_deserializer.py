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
    "data": [
      1
    ],
    "description": "one element matches, valid maxContains",
    "valid": true
  },
  {
    "data": [
      1,
      1
    ],
    "description": "too many elements match, invalid maxContains",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Maxcontains2Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

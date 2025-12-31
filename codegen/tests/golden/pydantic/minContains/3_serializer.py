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
    "data": [
      1
    ],
    "description": "one element matches, invalid minContains",
    "valid": false
  },
  {
    "data": [
      1,
      1
    ],
    "description": "both elements match, valid minContains",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Mincontains3Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

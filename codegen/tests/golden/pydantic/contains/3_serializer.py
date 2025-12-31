"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contains": false
}

Tests:
[
  {
    "data": [
      "foo"
    ],
    "description": "any non-empty array is invalid",
    "valid": false
  },
  {
    "data": [],
    "description": "empty array is invalid",
    "valid": false
  },
  {
    "data": "contains does not apply to strings",
    "description": "non-arrays are valid",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Contains3Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contains": true
}

Tests:
[
  {
    "data": [
      "foo"
    ],
    "description": "any non-empty array is valid",
    "valid": true
  },
  {
    "data": [],
    "description": "empty array is invalid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Contains2Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

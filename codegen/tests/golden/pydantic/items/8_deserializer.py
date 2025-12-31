"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": false,
  "prefixItems": [
    {}
  ]
}

Tests:
[
  {
    "data": [
      "foo",
      "bar",
      37
    ],
    "description": "heterogeneous invalid instance",
    "valid": false
  },
  {
    "data": [
      null
    ],
    "description": "valid instance",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Items8Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "prefixItems": [
    {
      "type": "integer"
    }
  ]
}

Tests:
[
  {
    "data": [
      1,
      "foo",
      false
    ],
    "description": "only the first item is validated",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Prefixitems2Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "prefixItems": [
    {
      "type": "string"
    }
  ],
  "unevaluatedItems": false
}

Tests:
[
  {
    "data": [
      "foo"
    ],
    "description": "with no unevaluated items",
    "valid": true
  },
  {
    "data": [
      "foo",
      "bar"
    ],
    "description": "with unevaluated items",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluateditems4Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

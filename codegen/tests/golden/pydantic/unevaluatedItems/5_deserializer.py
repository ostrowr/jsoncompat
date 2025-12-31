"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": true,
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
      "foo",
      42
    ],
    "description": "unevaluatedItems doesn't apply",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluateditems5Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

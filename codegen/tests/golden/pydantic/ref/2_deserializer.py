"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "prefixItems": [
    {
      "type": "integer"
    },
    {
      "$ref": "#/prefixItems/0"
    }
  ]
}

Tests:
[
  {
    "data": [
      1,
      2
    ],
    "description": "match array",
    "valid": true
  },
  {
    "data": [
      1,
      "foo"
    ],
    "description": "mismatch array",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Ref2Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

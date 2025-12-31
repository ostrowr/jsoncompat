"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "if": {
    "prefixItems": [
      {
        "const": "a"
      }
    ]
  },
  "unevaluatedItems": false
}

Tests:
[
  {
    "data": [
      "a"
    ],
    "description": "valid in case if is evaluated",
    "valid": true
  },
  {
    "data": [
      "b"
    ],
    "description": "invalid in case if is evaluated",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluateditems26Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")

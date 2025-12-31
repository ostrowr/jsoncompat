"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "prefixItems": [
        true,
        {
          "type": "number"
        }
      ]
    }
  ],
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
    "description": "with no unevaluated items",
    "valid": true
  },
  {
    "data": [
      "foo",
      42,
      true
    ],
    "description": "with unevaluated items",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluateditems7Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")

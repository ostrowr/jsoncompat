"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "prefixItems": [
        {
          "type": "string"
        }
      ]
    },
    {
      "unevaluatedItems": true
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
    "description": "with no additional items",
    "valid": true
  },
  {
    "data": [
      "foo",
      42,
      true
    ],
    "description": "with additional items",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluateditems10Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")

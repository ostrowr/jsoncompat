"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "properties": {
        "foo": {
          "prefixItems": [
            true,
            {
              "type": "string"
            }
          ]
        }
      }
    }
  ],
  "properties": {
    "foo": {
      "prefixItems": [
        {
          "type": "string"
        }
      ],
      "unevaluatedItems": false
    }
  }
}

Tests:
[
  {
    "data": {
      "foo": [
        "test"
      ]
    },
    "description": "no extra items",
    "valid": true
  },
  {
    "data": {
      "foo": [
        "test",
        "test"
      ]
    },
    "description": "uncle keyword evaluation is not significant",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluateditems20Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/1: allOf with non-object schema")

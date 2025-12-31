"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "prefixItems": [
        true,
        {
          "const": "bar"
        }
      ]
    },
    {
      "prefixItems": [
        true,
        true,
        {
          "const": "baz"
        }
      ]
    }
  ],
  "prefixItems": [
    {
      "const": "foo"
    }
  ],
  "unevaluatedItems": false
}

Tests:
[
  {
    "data": [
      "foo",
      "bar"
    ],
    "description": "when one schema matches and has no unevaluated items",
    "valid": true
  },
  {
    "data": [
      "foo",
      "bar",
      42
    ],
    "description": "when one schema matches and has unevaluated items",
    "valid": false
  },
  {
    "data": [
      "foo",
      "bar",
      "baz"
    ],
    "description": "when two schemas match and has no unevaluated items",
    "valid": true
  },
  {
    "data": [
      "foo",
      "bar",
      "baz",
      42
    ],
    "description": "when two schemas match and has unevaluated items",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluateditems11Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")

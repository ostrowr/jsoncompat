"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "prefixItems": [
      true,
      true,
      true,
      {
        "const": "else"
      }
    ]
  },
  "if": {
    "prefixItems": [
      true,
      {
        "const": "bar"
      }
    ]
  },
  "prefixItems": [
    {
      "const": "foo"
    }
  ],
  "then": {
    "prefixItems": [
      true,
      true,
      {
        "const": "then"
      }
    ]
  },
  "unevaluatedItems": false
}

Tests:
[
  {
    "data": [
      "foo",
      "bar",
      "then"
    ],
    "description": "when if matches and it has no unevaluated items",
    "valid": true
  },
  {
    "data": [
      "foo",
      "bar",
      "then",
      "else"
    ],
    "description": "when if matches and it has unevaluated items",
    "valid": false
  },
  {
    "data": [
      "foo",
      42,
      42,
      "else"
    ],
    "description": "when if doesn't match and it has no unevaluated items",
    "valid": true
  },
  {
    "data": [
      "foo",
      42,
      42,
      "else",
      42
    ],
    "description": "when if doesn't match and it has unevaluated items",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluateditems14Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")

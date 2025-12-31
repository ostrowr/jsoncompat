"""
Schema:
{
  "$defs": {
    "baseSchema": {
      "$comment": "unevaluatedItems comes first so it's more likely to catch bugs with implementations that are sensitive to keyword ordering",
      "$defs": {
        "defaultAddons": {
          "$comment": "Needed to satisfy the bookending requirement",
          "$dynamicAnchor": "addons"
        }
      },
      "$dynamicRef": "#addons",
      "$id": "./baseSchema",
      "prefixItems": [
        {
          "type": "string"
        }
      ],
      "type": "array",
      "unevaluatedItems": false
    },
    "derived": {
      "$dynamicAnchor": "addons",
      "prefixItems": [
        true,
        {
          "type": "string"
        }
      ]
    }
  },
  "$id": "https://example.com/unevaluated-items-with-dynamic-ref/derived",
  "$ref": "./baseSchema",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": [
      "foo",
      "bar"
    ],
    "description": "with no unevaluated items",
    "valid": true
  },
  {
    "data": [
      "foo",
      "bar",
      "baz"
    ],
    "description": "with unevaluated items",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluateditems18Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

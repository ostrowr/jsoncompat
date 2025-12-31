"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "prefixItems": [
    {
      "type": "integer"
    },
    {
      "type": "string"
    }
  ]
}

Tests:
[
  {
    "data": [
      1,
      "foo"
    ],
    "description": "correct types",
    "valid": true
  },
  {
    "data": [
      "foo",
      1
    ],
    "description": "wrong types",
    "valid": false
  },
  {
    "data": [
      1
    ],
    "description": "incomplete array of items",
    "valid": true
  },
  {
    "data": [
      1,
      "foo",
      true
    ],
    "description": "array with additional items",
    "valid": true
  },
  {
    "data": [],
    "description": "empty array",
    "valid": true
  },
  {
    "data": {
      "0": "invalid",
      "1": "valid",
      "length": 2
    },
    "description": "JavaScript pseudo-array is valid",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Prefixitems0Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

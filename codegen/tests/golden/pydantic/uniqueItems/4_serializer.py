"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "prefixItems": [
    {
      "type": "boolean"
    },
    {
      "type": "boolean"
    }
  ],
  "uniqueItems": false
}

Tests:
[
  {
    "data": [
      false,
      true
    ],
    "description": "[false, true] from items array is valid",
    "valid": true
  },
  {
    "data": [
      true,
      false
    ],
    "description": "[true, false] from items array is valid",
    "valid": true
  },
  {
    "data": [
      false,
      false
    ],
    "description": "[false, false] from items array is valid",
    "valid": true
  },
  {
    "data": [
      true,
      true
    ],
    "description": "[true, true] from items array is valid",
    "valid": true
  },
  {
    "data": [
      false,
      true,
      "foo",
      "bar"
    ],
    "description": "unique array extended from [false, true] is valid",
    "valid": true
  },
  {
    "data": [
      true,
      false,
      "foo",
      "bar"
    ],
    "description": "unique array extended from [true, false] is valid",
    "valid": true
  },
  {
    "data": [
      false,
      true,
      "foo",
      "foo"
    ],
    "description": "non-unique array extended from [false, true] is valid",
    "valid": true
  },
  {
    "data": [
      true,
      false,
      "foo",
      "foo"
    ],
    "description": "non-unique array extended from [true, false] is valid",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Uniqueitems4Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

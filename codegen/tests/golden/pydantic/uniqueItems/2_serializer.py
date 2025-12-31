"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": false,
  "prefixItems": [
    {
      "type": "boolean"
    },
    {
      "type": "boolean"
    }
  ],
  "uniqueItems": true
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
    "description": "[false, false] from items array is not valid",
    "valid": false
  },
  {
    "data": [
      true,
      true
    ],
    "description": "[true, true] from items array is not valid",
    "valid": false
  },
  {
    "data": [
      false,
      true,
      null
    ],
    "description": "extra items are invalid even if unique",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Uniqueitems2Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: prefixItems/contains")

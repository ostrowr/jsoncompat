"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    [
      1
    ]
  ]
}

Tests:
[
  {
    "data": [
      true
    ],
    "description": "[true] is invalid",
    "valid": false
  },
  {
    "data": [
      1
    ],
    "description": "[1] is valid",
    "valid": true
  },
  {
    "data": [
      1.0
    ],
    "description": "[1.0] is valid",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Enum12Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported enum/const value at #: [1]")

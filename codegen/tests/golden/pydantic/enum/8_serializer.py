"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    [
      true
    ]
  ]
}

Tests:
[
  {
    "data": [
      true
    ],
    "description": "[true] is valid",
    "valid": true
  },
  {
    "data": [
      1
    ],
    "description": "[1] is invalid",
    "valid": false
  },
  {
    "data": [
      1.0
    ],
    "description": "[1.0] is invalid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Enum8Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported enum/const value at #: [true]")

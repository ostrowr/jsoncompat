"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    [
      0
    ]
  ]
}

Tests:
[
  {
    "data": [
      false
    ],
    "description": "[false] is invalid",
    "valid": false
  },
  {
    "data": [
      0
    ],
    "description": "[0] is valid",
    "valid": true
  },
  {
    "data": [
      0.0
    ],
    "description": "[0.0] is valid",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Enum10Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported enum/const value at #: [0]")

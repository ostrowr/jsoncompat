"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    [
      false
    ]
  ]
}

Tests:
[
  {
    "data": [
      false
    ],
    "description": "[false] is valid",
    "valid": true
  },
  {
    "data": [
      0
    ],
    "description": "[0] is invalid",
    "valid": false
  },
  {
    "data": [
      0.0
    ],
    "description": "[0.0] is invalid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Enum6Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported enum/const value at #: [false]")

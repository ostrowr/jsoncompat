"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": {
    "a": true
  }
}

Tests:
[
  {
    "data": {
      "a": true
    },
    "description": "{\"a\": true} is valid",
    "valid": true
  },
  {
    "data": {
      "a": 1
    },
    "description": "{\"a\": 1} is invalid",
    "valid": false
  },
  {
    "data": {
      "a": 1.0
    },
    "description": "{\"a\": 1.0} is invalid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Const9Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported enum/const value at #: {\"a\":true}")

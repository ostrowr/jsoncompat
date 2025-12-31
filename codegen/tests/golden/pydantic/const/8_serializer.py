"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": {
    "a": false
  }
}

Tests:
[
  {
    "data": {
      "a": false
    },
    "description": "{\"a\": false} is valid",
    "valid": true
  },
  {
    "data": {
      "a": 0
    },
    "description": "{\"a\": 0} is invalid",
    "valid": false
  },
  {
    "data": {
      "a": 0.0
    },
    "description": "{\"a\": 0.0} is invalid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Const8Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported enum/const value at #: {\"a\":false}")

"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": [
    {
      "foo": "bar"
    }
  ]
}

Tests:
[
  {
    "data": [
      {
        "foo": "bar"
      }
    ],
    "description": "same array is valid",
    "valid": true
  },
  {
    "data": [
      2
    ],
    "description": "another array item is invalid",
    "valid": false
  },
  {
    "data": [
      1,
      2,
      3
    ],
    "description": "array with additional items is invalid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Const2Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported enum/const value at #: [{\"foo\":\"bar\"}]")

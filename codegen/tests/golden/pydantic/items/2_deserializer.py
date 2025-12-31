"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": false
}

Tests:
[
  {
    "data": [
      1,
      "foo",
      true
    ],
    "description": "any non-empty array is invalid",
    "valid": false
  },
  {
    "data": [],
    "description": "empty array is valid",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Items2Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/items: false schema")

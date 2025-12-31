"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    true,
    false,
    false
  ]
}

Tests:
[
  {
    "data": "foo",
    "description": "any value is valid",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Oneof3Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/oneOf/1: false schema")

"""
Schema:
{
  "$ref": "http://example.com/ref/if",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "if": {
    "$id": "http://example.com/ref/if",
    "type": "integer"
  }
}

Tests:
[
  {
    "data": "foo",
    "description": "a non-integer is invalid due to the $ref",
    "valid": false
  },
  {
    "data": 12,
    "description": "an integer is valid",
    "valid": true
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Ref28Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")

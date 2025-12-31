"""
Schema:
{
  "$ref": "http://example.com/ref/then",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "then": {
    "$id": "http://example.com/ref/then",
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

class Ref29Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")

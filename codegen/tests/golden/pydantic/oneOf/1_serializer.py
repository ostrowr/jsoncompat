"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "minLength": 2
    },
    {
      "maxLength": 4
    }
  ],
  "type": "string"
}

Tests:
[
  {
    "data": 3,
    "description": "mismatch base schema",
    "valid": false
  },
  {
    "data": "foobar",
    "description": "one oneOf valid",
    "valid": true
  },
  {
    "data": "foo",
    "description": "both oneOf valid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Oneof1Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")

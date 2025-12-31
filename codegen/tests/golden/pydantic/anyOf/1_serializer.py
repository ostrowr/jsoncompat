"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "maxLength": 2
    },
    {
      "minLength": 4
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
    "description": "one anyOf valid",
    "valid": true
  },
  {
    "data": "foo",
    "description": "both anyOf invalid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Anyof1Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")

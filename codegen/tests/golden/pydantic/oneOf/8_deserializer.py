"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "required": [
        "foo",
        "bar"
      ]
    },
    {
      "required": [
        "foo",
        "baz"
      ]
    }
  ],
  "type": "object"
}

Tests:
[
  {
    "data": {
      "bar": 2
    },
    "description": "both invalid - invalid",
    "valid": false
  },
  {
    "data": {
      "bar": 2,
      "foo": 1
    },
    "description": "first valid - valid",
    "valid": true
  },
  {
    "data": {
      "baz": 3,
      "foo": 1
    },
    "description": "second valid - valid",
    "valid": true
  },
  {
    "data": {
      "bar": 2,
      "baz": 3,
      "foo": 1
    },
    "description": "both valid - invalid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Oneof8Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/1: allOf with non-object schema")

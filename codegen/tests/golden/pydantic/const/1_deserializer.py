"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": {
    "baz": "bax",
    "foo": "bar"
  }
}

Tests:
[
  {
    "data": {
      "baz": "bax",
      "foo": "bar"
    },
    "description": "same object is valid",
    "valid": true
  },
  {
    "data": {
      "baz": "bax",
      "foo": "bar"
    },
    "description": "same object with different property order is valid",
    "valid": true
  },
  {
    "data": {
      "foo": "bar"
    },
    "description": "another object is invalid",
    "valid": false
  },
  {
    "data": [
      1,
      2
    ],
    "description": "another type is invalid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Const1Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported enum/const value at #: {\"baz\":\"bax\",\"foo\":\"bar\"}")

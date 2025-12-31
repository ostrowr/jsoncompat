"""
Schema:
false

Tests:
[
  {
    "data": 1,
    "description": "number is invalid",
    "valid": false
  },
  {
    "data": "foo",
    "description": "string is invalid",
    "valid": false
  },
  {
    "data": true,
    "description": "boolean true is invalid",
    "valid": false
  },
  {
    "data": false,
    "description": "boolean false is invalid",
    "valid": false
  },
  {
    "data": null,
    "description": "null is invalid",
    "valid": false
  },
  {
    "data": {
      "foo": "bar"
    },
    "description": "object is invalid",
    "valid": false
  },
  {
    "data": {},
    "description": "empty object is invalid",
    "valid": false
  },
  {
    "data": [
      "foo"
    ],
    "description": "array is invalid",
    "valid": false
  },
  {
    "data": [],
    "description": "empty array is invalid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class BooleanSchema1Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #: false schema")

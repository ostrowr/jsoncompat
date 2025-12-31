"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    6,
    "foo",
    [],
    true,
    {
      "foo": 12
    }
  ]
}

Tests:
[
  {
    "data": [],
    "description": "one of the enum is valid",
    "valid": true
  },
  {
    "data": null,
    "description": "something else is invalid",
    "valid": false
  },
  {
    "data": {
      "foo": false
    },
    "description": "objects are deep compared",
    "valid": false
  },
  {
    "data": {
      "foo": 12
    },
    "description": "valid object matches",
    "valid": true
  },
  {
    "data": {
      "boo": 42,
      "foo": 12
    },
    "description": "extra properties in object is invalid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Enum1Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported enum/const value at #: []")

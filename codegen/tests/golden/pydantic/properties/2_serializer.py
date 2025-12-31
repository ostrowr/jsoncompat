"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar": false,
    "foo": true
  }
}

Tests:
[
  {
    "data": {},
    "description": "no property present is valid",
    "valid": true
  },
  {
    "data": {
      "foo": 1
    },
    "description": "only 'true' property present is valid",
    "valid": true
  },
  {
    "data": {
      "bar": 2
    },
    "description": "only 'false' property present is invalid",
    "valid": false
  },
  {
    "data": {
      "bar": 2,
      "foo": 1
    },
    "description": "both properties present is invalid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Properties2Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/properties/bar: false schema")

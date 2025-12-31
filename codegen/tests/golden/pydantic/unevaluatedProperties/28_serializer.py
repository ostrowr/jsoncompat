"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "unevaluatedProperties": true
    },
    {
      "properties": {
        "foo": {
          "type": "string"
        }
      },
      "unevaluatedProperties": false
    }
  ],
  "type": "object"
}

Tests:
[
  {
    "data": {
      "foo": "foo"
    },
    "description": "with no nested unevaluated properties",
    "valid": true
  },
  {
    "data": {
      "bar": "bar",
      "foo": "foo"
    },
    "description": "with nested unevaluated properties",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluatedproperties28Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/1: allOf with non-object schema")

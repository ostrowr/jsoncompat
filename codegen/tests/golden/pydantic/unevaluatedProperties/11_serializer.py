"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "properties": {
        "bar": {
          "const": "bar"
        }
      },
      "required": [
        "bar"
      ]
    },
    {
      "properties": {
        "baz": {
          "const": "baz"
        }
      },
      "required": [
        "baz"
      ]
    }
  ],
  "properties": {
    "foo": {
      "type": "string"
    }
  },
  "type": "object",
  "unevaluatedProperties": false
}

Tests:
[
  {
    "data": {
      "bar": "bar",
      "foo": "foo"
    },
    "description": "with no unevaluated properties",
    "valid": true
  },
  {
    "data": {
      "bar": "bar",
      "foo": "foo",
      "quux": "quux"
    },
    "description": "with unevaluated properties",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluatedproperties11Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/1: allOf with non-object schema")

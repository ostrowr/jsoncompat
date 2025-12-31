"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "properties": {
        "foo": {
          "properties": {
            "faz": {
              "type": "string"
            }
          }
        }
      }
    }
  ],
  "properties": {
    "foo": {
      "properties": {
        "bar": {
          "type": "string"
        }
      },
      "type": "object",
      "unevaluatedProperties": false
    }
  },
  "type": "object"
}

Tests:
[
  {
    "data": {
      "foo": {
        "bar": "test"
      }
    },
    "description": "no extra properties",
    "valid": true
  },
  {
    "data": {
      "foo": {
        "bar": "test",
        "faz": "test"
      }
    },
    "description": "uncle keyword evaluation is not significant",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluatedproperties29Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/1: allOf with non-object schema")

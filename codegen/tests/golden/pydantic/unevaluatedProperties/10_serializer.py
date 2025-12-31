"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
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
    },
    {
      "properties": {
        "quux": {
          "const": "quux"
        }
      },
      "required": [
        "quux"
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
    "description": "when one matches and has no unevaluated properties",
    "valid": true
  },
  {
    "data": {
      "bar": "bar",
      "baz": "not-baz",
      "foo": "foo"
    },
    "description": "when one matches and has unevaluated properties",
    "valid": false
  },
  {
    "data": {
      "bar": "bar",
      "baz": "baz",
      "foo": "foo"
    },
    "description": "when two match and has no unevaluated properties",
    "valid": true
  },
  {
    "data": {
      "bar": "bar",
      "baz": "baz",
      "foo": "foo",
      "quux": "not-quux"
    },
    "description": "when two match and has unevaluated properties",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluatedproperties10Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/1: allOf with non-object schema")

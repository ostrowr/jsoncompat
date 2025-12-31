"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "properties": {
      "baz": {
        "type": "string"
      }
    },
    "required": [
      "baz"
    ]
  },
  "if": {
    "properties": {
      "foo": {
        "const": "then"
      }
    },
    "required": [
      "foo"
    ]
  },
  "then": {
    "properties": {
      "bar": {
        "type": "string"
      }
    },
    "required": [
      "bar"
    ]
  },
  "type": "object",
  "unevaluatedProperties": false
}

Tests:
[
  {
    "data": {
      "bar": "bar",
      "foo": "then"
    },
    "description": "when if is true and has no unevaluated properties",
    "valid": true
  },
  {
    "data": {
      "bar": "bar",
      "baz": "baz",
      "foo": "then"
    },
    "description": "when if is true and has unevaluated properties",
    "valid": false
  },
  {
    "data": {
      "baz": "baz"
    },
    "description": "when if is false and has no unevaluated properties",
    "valid": true
  },
  {
    "data": {
      "baz": "baz",
      "foo": "else"
    },
    "description": "when if is false and has unevaluated properties",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluatedproperties13Deserializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/1: allOf with non-object schema")

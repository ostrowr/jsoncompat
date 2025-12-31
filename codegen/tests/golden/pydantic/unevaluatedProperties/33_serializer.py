"""
Schema:
{
  "$defs": {
    "one": {
      "properties": {
        "a": true
      }
    },
    "two": {
      "properties": {
        "x": true
      },
      "required": [
        "x"
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "$ref": "#/$defs/one"
    },
    {
      "properties": {
        "b": true
      }
    },
    {
      "oneOf": [
        {
          "$ref": "#/$defs/two"
        },
        {
          "properties": {
            "y": true
          },
          "required": [
            "y"
          ]
        }
      ]
    }
  ],
  "unevaluatedProperties": false
}

Tests:
[
  {
    "data": {},
    "description": "Empty is invalid (no x or y)",
    "valid": false
  },
  {
    "data": {
      "a": 1,
      "b": 1
    },
    "description": "a and b are invalid (no x or y)",
    "valid": false
  },
  {
    "data": {
      "x": 1,
      "y": 1
    },
    "description": "x and y are invalid",
    "valid": false
  },
  {
    "data": {
      "a": 1,
      "x": 1
    },
    "description": "a and x are valid",
    "valid": true
  },
  {
    "data": {
      "a": 1,
      "y": 1
    },
    "description": "a and y are valid",
    "valid": true
  },
  {
    "data": {
      "a": 1,
      "b": 1,
      "x": 1
    },
    "description": "a and b and x are valid",
    "valid": true
  },
  {
    "data": {
      "a": 1,
      "b": 1,
      "y": 1
    },
    "description": "a and b and y are valid",
    "valid": true
  },
  {
    "data": {
      "a": 1,
      "b": 1,
      "x": 1,
      "y": 1
    },
    "description": "a and b and x and y are invalid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluatedproperties33Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")

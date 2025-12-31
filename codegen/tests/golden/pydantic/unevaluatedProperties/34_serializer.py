"""
Schema:
{
  "$defs": {
    "one": {
      "oneOf": [
        {
          "$ref": "#/$defs/two"
        },
        {
          "properties": {
            "b": true
          },
          "required": [
            "b"
          ]
        },
        {
          "patternProperties": {
            "x": true
          },
          "required": [
            "xx"
          ]
        },
        {
          "required": [
            "all"
          ],
          "unevaluatedProperties": true
        }
      ]
    },
    "two": {
      "oneOf": [
        {
          "properties": {
            "c": true
          },
          "required": [
            "c"
          ]
        },
        {
          "properties": {
            "d": true
          },
          "required": [
            "d"
          ]
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "$ref": "#/$defs/one"
    },
    {
      "properties": {
        "a": true
      },
      "required": [
        "a"
      ]
    }
  ],
  "unevaluatedProperties": false
}

Tests:
[
  {
    "data": {},
    "description": "Empty is invalid",
    "valid": false
  },
  {
    "data": {
      "a": 1
    },
    "description": "a is valid",
    "valid": true
  },
  {
    "data": {
      "b": 1
    },
    "description": "b is valid",
    "valid": true
  },
  {
    "data": {
      "c": 1
    },
    "description": "c is valid",
    "valid": true
  },
  {
    "data": {
      "d": 1
    },
    "description": "d is valid",
    "valid": true
  },
  {
    "data": {
      "a": 1,
      "b": 1
    },
    "description": "a + b is invalid",
    "valid": false
  },
  {
    "data": {
      "a": 1,
      "c": 1
    },
    "description": "a + c is invalid",
    "valid": false
  },
  {
    "data": {
      "a": 1,
      "d": 1
    },
    "description": "a + d is invalid",
    "valid": false
  },
  {
    "data": {
      "b": 1,
      "c": 1
    },
    "description": "b + c is invalid",
    "valid": false
  },
  {
    "data": {
      "b": 1,
      "d": 1
    },
    "description": "b + d is invalid",
    "valid": false
  },
  {
    "data": {
      "c": 1,
      "d": 1
    },
    "description": "c + d is invalid",
    "valid": false
  },
  {
    "data": {
      "xx": 1
    },
    "description": "xx is valid",
    "valid": true
  },
  {
    "data": {
      "foox": 1,
      "xx": 1
    },
    "description": "xx + foox is valid",
    "valid": true
  },
  {
    "data": {
      "foo": 1,
      "xx": 1
    },
    "description": "xx + foo is invalid",
    "valid": false
  },
  {
    "data": {
      "a": 1,
      "xx": 1
    },
    "description": "xx + a is invalid",
    "valid": false
  },
  {
    "data": {
      "b": 1,
      "xx": 1
    },
    "description": "xx + b is invalid",
    "valid": false
  },
  {
    "data": {
      "c": 1,
      "xx": 1
    },
    "description": "xx + c is invalid",
    "valid": false
  },
  {
    "data": {
      "d": 1,
      "xx": 1
    },
    "description": "xx + d is invalid",
    "valid": false
  },
  {
    "data": {
      "all": 1
    },
    "description": "all is valid",
    "valid": true
  },
  {
    "data": {
      "all": 1,
      "foo": 1
    },
    "description": "all + foo is valid",
    "valid": true
  },
  {
    "data": {
      "a": 1,
      "all": 1
    },
    "description": "all + a is invalid",
    "valid": false
  }
]
"""

from pydantic import BaseModel, ConfigDict

class Unevaluatedproperties34Serializer(BaseModel):
    model_config = ConfigDict(extra="forbid")

    @classmethod
    def __get_pydantic_core_schema__(cls, source, handler):
        raise NotImplementedError("unsupported schema feature at #/allOf/0: allOf with non-object schema")

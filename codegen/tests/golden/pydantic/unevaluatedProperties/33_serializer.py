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

from typing import ClassVar

from jsonschema_rs import validator_for
from pydantic import BaseModel, ConfigDict, model_validator

_JSON_SCHEMA = r"""
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
"""
_VALIDATE_FORMATS = False

class Unevaluatedproperties33Serializer(BaseModel):
    __json_schema__: ClassVar[str] = _JSON_SCHEMA
    _jsonschema_validator: ClassVar[object | None] = None

    @classmethod
    def _get_jsonschema_validator(cls):
        validator = cls._jsonschema_validator
        if validator is None:
            validator = validator_for(cls.__json_schema__, validate_formats=_VALIDATE_FORMATS)
            cls._jsonschema_validator = validator
        return validator

    @model_validator(mode="before")
    @classmethod
    def _validate_jsonschema(cls, value):
        cls._get_jsonschema_validator().validate(value)
        return value

    model_config = ConfigDict(extra="forbid")
    __json_compat_error__: ClassVar[str] = "unsupported schema feature at #/allOf/0: allOf with non-object schema"

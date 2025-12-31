"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "multipleOf": 2
    }
  ],
  "anyOf": [
    {
      "multipleOf": 3
    }
  ],
  "oneOf": [
    {
      "multipleOf": 5
    }
  ]
}

Tests:
[
  {
    "data": 1,
    "description": "allOf: false, anyOf: false, oneOf: false",
    "valid": false
  },
  {
    "data": 5,
    "description": "allOf: false, anyOf: false, oneOf: true",
    "valid": false
  },
  {
    "data": 3,
    "description": "allOf: false, anyOf: true, oneOf: false",
    "valid": false
  },
  {
    "data": 15,
    "description": "allOf: false, anyOf: true, oneOf: true",
    "valid": false
  },
  {
    "data": 2,
    "description": "allOf: true, anyOf: false, oneOf: false",
    "valid": false
  },
  {
    "data": 10,
    "description": "allOf: true, anyOf: false, oneOf: true",
    "valid": false
  },
  {
    "data": 6,
    "description": "allOf: true, anyOf: true, oneOf: false",
    "valid": false
  },
  {
    "data": 30,
    "description": "allOf: true, anyOf: true, oneOf: true",
    "valid": true
  }
]
"""

from typing import ClassVar

from jsonschema_rs import validator_for
from pydantic import BaseModel, ConfigDict, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "multipleOf": 2
    }
  ],
  "anyOf": [
    {
      "multipleOf": 3
    }
  ],
  "oneOf": [
    {
      "multipleOf": 5
    }
  ]
}
"""
_VALIDATE_FORMATS = False

class Allof11Deserializer(BaseModel):
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

"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "required": [
        "foo",
        "bar"
      ]
    },
    {
      "required": [
        "foo",
        "baz"
      ]
    }
  ],
  "type": "object"
}

Tests:
[
  {
    "data": {
      "bar": 2
    },
    "description": "both invalid - invalid",
    "valid": false
  },
  {
    "data": {
      "bar": 2,
      "foo": 1
    },
    "description": "first valid - valid",
    "valid": true
  },
  {
    "data": {
      "baz": 3,
      "foo": 1
    },
    "description": "second valid - valid",
    "valid": true
  },
  {
    "data": {
      "bar": 2,
      "baz": 3,
      "foo": 1
    },
    "description": "both valid - invalid",
    "valid": false
  }
]
"""

from typing import ClassVar

from jsonschema_rs import validator_for
from pydantic import BaseModel, ConfigDict, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "required": [
        "foo",
        "bar"
      ]
    },
    {
      "required": [
        "foo",
        "baz"
      ]
    }
  ],
  "type": "object"
}
"""
_VALIDATE_FORMATS = False

class Oneof8Serializer(BaseModel):
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
    __json_compat_error__: ClassVar[str] = "unsupported schema feature at #/allOf/1: allOf with non-object schema"

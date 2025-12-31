"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "contains": {
        "multipleOf": 2
      }
    },
    {
      "contains": {
        "multipleOf": 3
      }
    }
  ],
  "unevaluatedItems": {
    "multipleOf": 5
  }
}

Tests:
[
  {
    "data": [
      2,
      3,
      4,
      5,
      6
    ],
    "description": "5 not evaluated, passes unevaluatedItems",
    "valid": true
  },
  {
    "data": [
      2,
      3,
      4,
      7,
      8
    ],
    "description": "7 not evaluated, fails unevaluatedItems",
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
  "allOf": [
    {
      "contains": {
        "multipleOf": 2
      }
    },
    {
      "contains": {
        "multipleOf": 3
      }
    }
  ],
  "unevaluatedItems": {
    "multipleOf": 5
  }
}
"""
_VALIDATE_FORMATS = False

class Unevaluateditems22Serializer(BaseModel):
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

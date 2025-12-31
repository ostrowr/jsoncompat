"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": false,
  "prefixItems": [
    {
      "type": "boolean"
    },
    {
      "type": "boolean"
    }
  ],
  "uniqueItems": true
}

Tests:
[
  {
    "data": [
      false,
      true
    ],
    "description": "[false, true] from items array is valid",
    "valid": true
  },
  {
    "data": [
      true,
      false
    ],
    "description": "[true, false] from items array is valid",
    "valid": true
  },
  {
    "data": [
      false,
      false
    ],
    "description": "[false, false] from items array is not valid",
    "valid": false
  },
  {
    "data": [
      true,
      true
    ],
    "description": "[true, true] from items array is not valid",
    "valid": false
  },
  {
    "data": [
      false,
      true,
      null
    ],
    "description": "extra items are invalid even if unique",
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
  "items": false,
  "prefixItems": [
    {
      "type": "boolean"
    },
    {
      "type": "boolean"
    }
  ],
  "uniqueItems": true
}
"""
_VALIDATE_FORMATS = False

class Uniqueitems2Deserializer(BaseModel):
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
    __json_compat_error__: ClassVar[str] = "unsupported schema feature at #: prefixItems/contains"

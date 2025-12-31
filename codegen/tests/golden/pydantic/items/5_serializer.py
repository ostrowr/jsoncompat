"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": false,
  "prefixItems": [
    {},
    {},
    {}
  ]
}

Tests:
[
  {
    "data": [],
    "description": "empty array",
    "valid": true
  },
  {
    "data": [
      1
    ],
    "description": "fewer number of items present (1)",
    "valid": true
  },
  {
    "data": [
      1,
      2
    ],
    "description": "fewer number of items present (2)",
    "valid": true
  },
  {
    "data": [
      1,
      2,
      3
    ],
    "description": "equal number of items present",
    "valid": true
  },
  {
    "data": [
      1,
      2,
      3,
      4
    ],
    "description": "additional items are not permitted",
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
    {},
    {},
    {}
  ]
}
"""
_VALIDATE_FORMATS = False

class Items5Serializer(BaseModel):
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

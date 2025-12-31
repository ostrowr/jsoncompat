"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contains": {
    "multipleOf": 3
  },
  "items": {
    "multipleOf": 2
  }
}

Tests:
[
  {
    "data": [
      2,
      4,
      8
    ],
    "description": "matches items, does not match contains",
    "valid": false
  },
  {
    "data": [
      3,
      6,
      9
    ],
    "description": "does not match items, matches contains",
    "valid": false
  },
  {
    "data": [
      6,
      12
    ],
    "description": "matches both items and contains",
    "valid": true
  },
  {
    "data": [
      1,
      5
    ],
    "description": "matches neither items nor contains",
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
  "contains": {
    "multipleOf": 3
  },
  "items": {
    "multipleOf": 2
  }
}
"""
_VALIDATE_FORMATS = False

class Contains4Deserializer(BaseModel):
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

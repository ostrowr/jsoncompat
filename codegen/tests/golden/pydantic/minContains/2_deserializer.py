"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contains": {
    "const": 1
  },
  "minContains": 2
}

Tests:
[
  {
    "data": [],
    "description": "empty data",
    "valid": false
  },
  {
    "data": [
      1
    ],
    "description": "all elements match, invalid minContains",
    "valid": false
  },
  {
    "data": [
      1,
      2
    ],
    "description": "some elements match, invalid minContains",
    "valid": false
  },
  {
    "data": [
      1,
      1
    ],
    "description": "all elements match, valid minContains (exactly as needed)",
    "valid": true
  },
  {
    "data": [
      1,
      1,
      1
    ],
    "description": "all elements match, valid minContains (more than needed)",
    "valid": true
  },
  {
    "data": [
      1,
      2,
      1
    ],
    "description": "some elements match, valid minContains",
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
  "contains": {
    "const": 1
  },
  "minContains": 2
}
"""
_VALIDATE_FORMATS = False

class Mincontains2Deserializer(BaseModel):
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

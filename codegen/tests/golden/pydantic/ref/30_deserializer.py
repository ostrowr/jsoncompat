"""
Schema:
{
  "$ref": "http://example.com/ref/else",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "$id": "http://example.com/ref/else",
    "type": "integer"
  }
}

Tests:
[
  {
    "data": "foo",
    "description": "a non-integer is invalid due to the $ref",
    "valid": false
  },
  {
    "data": 12,
    "description": "an integer is valid",
    "valid": true
  }
]
"""

from typing import ClassVar

from jsonschema_rs import validator_for
from pydantic import BaseModel, ConfigDict, model_validator

_JSON_SCHEMA = r"""
{
  "$ref": "http://example.com/ref/else",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "else": {
    "$id": "http://example.com/ref/else",
    "type": "integer"
  }
}
"""
_VALIDATE_FORMATS = False

class Ref30Deserializer(BaseModel):
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

"""
Schema:
{
  "$ref": "http://localhost:1234/draft2019-09/ignore-prefixItems.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "array"
}

Tests:
[
  {
    "comment": "if the implementation is not processing the $ref as a 2019-09 schema, this test will fail",
    "data": [
      1,
      2,
      3
    ],
    "description": "first item not a string is valid",
    "valid": true
  }
]
"""

from typing import ClassVar

from jsonschema_rs import validator_for
from pydantic import BaseModel, ConfigDict, model_validator

_JSON_SCHEMA = r"""
{
  "$ref": "http://localhost:1234/draft2019-09/ignore-prefixItems.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "array"
}
"""
_VALIDATE_FORMATS = False

class CrossDraft0Serializer(BaseModel):
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

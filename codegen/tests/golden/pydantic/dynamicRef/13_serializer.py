"""
Schema:
{
  "$dynamicAnchor": "node",
  "$id": "http://localhost:1234/draft2020-12/strict-tree.json",
  "$ref": "tree.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "unevaluatedProperties": false
}

Tests:
[
  {
    "data": {
      "children": [
        {
          "daat": 1
        }
      ]
    },
    "description": "instance with misspelled field",
    "valid": false
  },
  {
    "data": {
      "children": [
        {
          "data": 1
        }
      ]
    },
    "description": "instance with correct field",
    "valid": true
  }
]
"""

from typing import ClassVar

from jsonschema_rs import validator_for
from pydantic import BaseModel, ConfigDict, model_validator

_JSON_SCHEMA = r"""
{
  "$dynamicAnchor": "node",
  "$id": "http://localhost:1234/draft2020-12/strict-tree.json",
  "$ref": "tree.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "unevaluatedProperties": false
}
"""
_VALIDATE_FORMATS = False

class Dynamicref13Serializer(BaseModel):
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

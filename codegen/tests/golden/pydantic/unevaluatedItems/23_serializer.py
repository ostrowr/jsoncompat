"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "if": {
    "contains": {
      "const": "a"
    }
  },
  "then": {
    "if": {
      "contains": {
        "const": "b"
      }
    },
    "then": {
      "if": {
        "contains": {
          "const": "c"
        }
      }
    }
  },
  "unevaluatedItems": false
}

Tests:
[
  {
    "data": [],
    "description": "empty array is valid",
    "valid": true
  },
  {
    "data": [
      "a",
      "a"
    ],
    "description": "only a's are valid",
    "valid": true
  },
  {
    "data": [
      "a",
      "b",
      "a",
      "b",
      "a"
    ],
    "description": "a's and b's are valid",
    "valid": true
  },
  {
    "data": [
      "c",
      "a",
      "c",
      "c",
      "b",
      "a"
    ],
    "description": "a's, b's and c's are valid",
    "valid": true
  },
  {
    "data": [
      "b",
      "b"
    ],
    "description": "only b's are invalid",
    "valid": false
  },
  {
    "data": [
      "c",
      "c"
    ],
    "description": "only c's are invalid",
    "valid": false
  },
  {
    "data": [
      "c",
      "b",
      "c",
      "b",
      "c"
    ],
    "description": "only b's and c's are invalid",
    "valid": false
  },
  {
    "data": [
      "c",
      "a",
      "c",
      "a",
      "c"
    ],
    "description": "only a's and c's are invalid",
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
  "if": {
    "contains": {
      "const": "a"
    }
  },
  "then": {
    "if": {
      "contains": {
        "const": "b"
      }
    },
    "then": {
      "if": {
        "contains": {
          "const": "c"
        }
      }
    }
  },
  "unevaluatedItems": false
}
"""
_VALIDATE_FORMATS = False

class Unevaluateditems23Serializer(BaseModel):
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

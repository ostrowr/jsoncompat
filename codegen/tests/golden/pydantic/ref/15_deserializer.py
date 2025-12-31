"""
Schema:
{
  "$id": "http://example.com/schema-relative-uri-defs1.json",
  "$ref": "schema-relative-uri-defs2.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "$defs": {
        "inner": {
          "properties": {
            "bar": {
              "type": "string"
            }
          }
        }
      },
      "$id": "schema-relative-uri-defs2.json",
      "$ref": "#/$defs/inner"
    }
  }
}

Tests:
[
  {
    "data": {
      "bar": "a",
      "foo": {
        "bar": 1
      }
    },
    "description": "invalid on inner field",
    "valid": false
  },
  {
    "data": {
      "bar": 1,
      "foo": {
        "bar": "a"
      }
    },
    "description": "invalid on outer field",
    "valid": false
  },
  {
    "data": {
      "bar": "a",
      "foo": {
        "bar": "a"
      }
    },
    "description": "valid on both fields",
    "valid": true
  }
]
"""

from typing import ClassVar

from jsonschema_rs import validator_for
from pydantic import BaseModel, ConfigDict, model_validator

_JSON_SCHEMA = r"""
{
  "$id": "http://example.com/schema-relative-uri-defs1.json",
  "$ref": "schema-relative-uri-defs2.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "$defs": {
        "inner": {
          "properties": {
            "bar": {
              "type": "string"
            }
          }
        }
      },
      "$id": "schema-relative-uri-defs2.json",
      "$ref": "#/$defs/inner"
    }
  }
}
"""
_VALIDATE_FORMATS = False

class Ref15Deserializer(BaseModel):
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

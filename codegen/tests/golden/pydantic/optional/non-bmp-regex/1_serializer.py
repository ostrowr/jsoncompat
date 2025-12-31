"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "patternProperties": {
    "^🐲*$": {
      "type": "integer"
    }
  }
}

Tests:
[
  {
    "data": {
      "": 1
    },
    "description": "matches empty",
    "valid": true
  },
  {
    "data": {
      "🐲": 1
    },
    "description": "matches single",
    "valid": true
  },
  {
    "data": {
      "🐲🐲": 1
    },
    "description": "matches two",
    "valid": true
  },
  {
    "data": {
      "🐲": "hello"
    },
    "description": "doesn't match one",
    "valid": false
  },
  {
    "data": {
      "🐲🐲": "hello"
    },
    "description": "doesn't match two",
    "valid": false
  }
]
"""

from typing import Any, ClassVar

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field, model_validator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "patternProperties": {
    "^🐲*$": {
      "type": "integer"
    }
  }
}
"""

_VALIDATE_FORMATS = False

class Nonbmpregex1Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


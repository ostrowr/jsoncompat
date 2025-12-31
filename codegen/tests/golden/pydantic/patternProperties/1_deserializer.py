"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "patternProperties": {
    "a*": {
      "type": "integer"
    },
    "aaa*": {
      "maximum": 20
    }
  }
}

Tests:
[
  {
    "data": {
      "a": 21
    },
    "description": "a single valid match is valid",
    "valid": true
  },
  {
    "data": {
      "aaaa": 18
    },
    "description": "a simultaneous match is valid",
    "valid": true
  },
  {
    "data": {
      "a": 21,
      "aaaa": 18
    },
    "description": "multiple matches is valid",
    "valid": true
  },
  {
    "data": {
      "a": "bar"
    },
    "description": "an invalid due to one is invalid",
    "valid": false
  },
  {
    "data": {
      "aaaa": 31
    },
    "description": "an invalid due to the other is invalid",
    "valid": false
  },
  {
    "data": {
      "aaa": "foo",
      "aaaa": 31
    },
    "description": "an invalid due to both is invalid",
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
    "a*": {
      "type": "integer"
    },
    "aaa*": {
      "maximum": 20
    }
  }
}
"""

_VALIDATE_FORMATS = False

class Patternproperties1Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


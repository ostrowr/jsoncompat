"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "patternProperties": {
    "f.*o": {
      "type": "integer"
    }
  }
}

Tests:
[
  {
    "data": {
      "foo": 1
    },
    "description": "a single valid match is valid",
    "valid": true
  },
  {
    "data": {
      "foo": 1,
      "foooooo": 2
    },
    "description": "multiple valid matches is valid",
    "valid": true
  },
  {
    "data": {
      "foo": "bar",
      "fooooo": 2
    },
    "description": "a single invalid match is invalid",
    "valid": false
  },
  {
    "data": {
      "foo": "bar",
      "foooooo": "baz"
    },
    "description": "multiple invalid matches is invalid",
    "valid": false
  },
  {
    "data": [
      "foo"
    ],
    "description": "ignores arrays",
    "valid": true
  },
  {
    "data": "foo",
    "description": "ignores strings",
    "valid": true
  },
  {
    "data": 12,
    "description": "ignores other non-objects",
    "valid": true
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
    "f.*o": {
      "type": "integer"
    }
  }
}
"""

_VALIDATE_FORMATS = False

class Patternproperties0Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


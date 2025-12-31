"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "dependentRequired": {
    "foo\nbar": [
      "foo\rbar"
    ],
    "foo\"bar": [
      "foo'bar"
    ]
  }
}

Tests:
[
  {
    "data": {
      "foo\nbar": 1,
      "foo\rbar": 2
    },
    "description": "CRLF",
    "valid": true
  },
  {
    "data": {
      "foo\"bar": 2,
      "foo'bar": 1
    },
    "description": "quoted quotes",
    "valid": true
  },
  {
    "data": {
      "foo": 2,
      "foo\nbar": 1
    },
    "description": "CRLF missing dependent",
    "valid": false
  },
  {
    "data": {
      "foo\"bar": 2
    },
    "description": "quoted quotes missing dependent",
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
  "dependentRequired": {
    "foo\nbar": [
      "foo\rbar"
    ],
    "foo\"bar": [
      "foo'bar"
    ]
  }
}
"""

_VALIDATE_FORMATS = False

class Dependentrequired3Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


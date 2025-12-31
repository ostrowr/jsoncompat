"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "dependencies": {
    "quux": [
      "foo",
      "bar"
    ]
  }
}

Tests:
[
  {
    "data": {},
    "description": "neither",
    "valid": true
  },
  {
    "data": {
      "bar": 2,
      "foo": 1
    },
    "description": "nondependants",
    "valid": true
  },
  {
    "data": {
      "bar": 2,
      "foo": 1,
      "quux": 3
    },
    "description": "with dependencies",
    "valid": true
  },
  {
    "data": {
      "foo": 1,
      "quux": 2
    },
    "description": "missing dependency",
    "valid": false
  },
  {
    "data": {
      "bar": 1,
      "quux": 2
    },
    "description": "missing other dependency",
    "valid": false
  },
  {
    "data": {
      "quux": 1
    },
    "description": "missing both dependencies",
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
  "dependencies": {
    "quux": [
      "foo",
      "bar"
    ]
  }
}
"""

_VALIDATE_FORMATS = False

class Dependenciescompatibility2Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


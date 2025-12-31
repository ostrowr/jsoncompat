"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "unevaluatedItems": false
}

Tests:
[
  {
    "data": true,
    "description": "ignores booleans",
    "valid": true
  },
  {
    "data": 123,
    "description": "ignores integers",
    "valid": true
  },
  {
    "data": 1.0,
    "description": "ignores floats",
    "valid": true
  },
  {
    "data": {},
    "description": "ignores objects",
    "valid": true
  },
  {
    "data": "foo",
    "description": "ignores strings",
    "valid": true
  },
  {
    "data": null,
    "description": "ignores null",
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
  "unevaluatedItems": false
}
"""

_VALIDATE_FORMATS = False

class Unevaluateditems24Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


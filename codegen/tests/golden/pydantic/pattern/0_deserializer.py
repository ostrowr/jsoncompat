"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^a*$"
}

Tests:
[
  {
    "data": "aaa",
    "description": "a matching pattern is valid",
    "valid": true
  },
  {
    "data": "abc",
    "description": "a non-matching pattern is invalid",
    "valid": false
  },
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
    "data": [],
    "description": "ignores arrays",
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
from pydantic import ConfigDict, Field, TypeAdapter, model_validator
from pydantic.functional_validators import BeforeValidator

_JSON_SCHEMA = r"""
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^a*$"
}
"""

_VALIDATE_FORMATS = False

class Pattern0Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "uri-reference"
}

Tests:
[
  {
    "data": 12,
    "description": "all string formats ignore integers",
    "valid": true
  },
  {
    "data": 13.7,
    "description": "all string formats ignore floats",
    "valid": true
  },
  {
    "data": {},
    "description": "all string formats ignore objects",
    "valid": true
  },
  {
    "data": [],
    "description": "all string formats ignore arrays",
    "valid": true
  },
  {
    "data": false,
    "description": "all string formats ignore booleans",
    "valid": true
  },
  {
    "data": null,
    "description": "all string formats ignore nulls",
    "valid": true
  },
  {
    "data": "http://foo.bar/?baz=qux#quux",
    "description": "a valid URI",
    "valid": true
  },
  {
    "data": "//foo.bar/?baz=qux#quux",
    "description": "a valid protocol-relative URI Reference",
    "valid": true
  },
  {
    "data": "/abc",
    "description": "a valid relative URI Reference",
    "valid": true
  },
  {
    "data": "\\\\WINDOWS\\fileshare",
    "description": "an invalid URI Reference",
    "valid": false
  },
  {
    "data": "abc",
    "description": "a valid URI Reference",
    "valid": true
  },
  {
    "data": "#fragment",
    "description": "a valid URI fragment",
    "valid": true
  },
  {
    "data": "#frag\\ment",
    "description": "an invalid URI fragment",
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
  "format": "uri-reference"
}
"""

_VALIDATE_FORMATS = False

class Urireference0Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


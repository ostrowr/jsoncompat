"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "uri-template"
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
    "data": "http://example.com/dictionary/{term:1}/{term}",
    "description": "a valid uri-template",
    "valid": true
  },
  {
    "data": "http://example.com/dictionary/{term:1}/{term",
    "description": "an invalid uri-template",
    "valid": false
  },
  {
    "data": "http://example.com/dictionary",
    "description": "a valid uri-template without variables",
    "valid": true
  },
  {
    "data": "dictionary/{term:1}/{term}",
    "description": "a valid relative uri-template",
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
  "format": "uri-template"
}
"""

_VALIDATE_FORMATS = False

class Uritemplate0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "relative-json-pointer"
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
    "data": "1",
    "description": "a valid upwards RJP",
    "valid": true
  },
  {
    "data": "0/foo/bar",
    "description": "a valid downwards RJP",
    "valid": true
  },
  {
    "data": "2/0/baz/1/zip",
    "description": "a valid up and then down RJP, with array index",
    "valid": true
  },
  {
    "data": "0#",
    "description": "a valid RJP taking the member or index name",
    "valid": true
  },
  {
    "data": "/foo/bar",
    "description": "an invalid RJP that is a valid JSON Pointer",
    "valid": false
  },
  {
    "data": "-1/foo/bar",
    "description": "negative prefix",
    "valid": false
  },
  {
    "data": "+1/foo/bar",
    "description": "explicit positive prefix",
    "valid": false
  },
  {
    "data": "0##",
    "description": "## is not a valid json-pointer",
    "valid": false
  },
  {
    "data": "01/a",
    "description": "zero cannot be followed by other digits, plus json-pointer",
    "valid": false
  },
  {
    "data": "01#",
    "description": "zero cannot be followed by other digits, plus octothorpe",
    "valid": false
  },
  {
    "data": "",
    "description": "empty string",
    "valid": false
  },
  {
    "data": "120/foo/bar",
    "description": "multi-digit integer prefix",
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
  "format": "relative-json-pointer"
}
"""

_VALIDATE_FORMATS = False

class Relativejsonpointer0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


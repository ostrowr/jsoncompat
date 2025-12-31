"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "iri-reference"
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
    "data": "http://ƒøø.ßår/?∂éœ=πîx#πîüx",
    "description": "a valid IRI",
    "valid": true
  },
  {
    "data": "//ƒøø.ßår/?∂éœ=πîx#πîüx",
    "description": "a valid protocol-relative IRI Reference",
    "valid": true
  },
  {
    "data": "/âππ",
    "description": "a valid relative IRI Reference",
    "valid": true
  },
  {
    "data": "\\\\WINDOWS\\filëßåré",
    "description": "an invalid IRI Reference",
    "valid": false
  },
  {
    "data": "âππ",
    "description": "a valid IRI Reference",
    "valid": true
  },
  {
    "data": "#ƒrägmênt",
    "description": "a valid IRI fragment",
    "valid": true
  },
  {
    "data": "#ƒräg\\mênt",
    "description": "an invalid IRI fragment",
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
  "format": "iri-reference"
}
"""

_VALIDATE_FORMATS = False

class Irireference0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


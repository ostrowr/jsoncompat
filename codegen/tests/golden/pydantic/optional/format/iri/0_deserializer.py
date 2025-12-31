"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "iri"
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
    "description": "a valid IRI with anchor tag",
    "valid": true
  },
  {
    "data": "http://ƒøø.com/blah_(wîkïpédiå)_blah#ßité-1",
    "description": "a valid IRI with anchor tag and parentheses",
    "valid": true
  },
  {
    "data": "http://ƒøø.ßår/?q=Test%20URL-encoded%20stuff",
    "description": "a valid IRI with URL-encoded stuff",
    "valid": true
  },
  {
    "data": "http://-.~_!$&'()*+,;=:%40:80%2f::::::@example.com",
    "description": "a valid IRI with many special characters",
    "valid": true
  },
  {
    "data": "http://[2001:0db8:85a3:0000:0000:8a2e:0370:7334]",
    "description": "a valid IRI based on IPv6",
    "valid": true
  },
  {
    "data": "http://2001:0db8:85a3:0000:0000:8a2e:0370:7334",
    "description": "an invalid IRI based on IPv6",
    "valid": false
  },
  {
    "data": "/abc",
    "description": "an invalid relative IRI Reference",
    "valid": false
  },
  {
    "data": "\\\\WINDOWS\\filëßåré",
    "description": "an invalid IRI",
    "valid": false
  },
  {
    "data": "âππ",
    "description": "an invalid IRI though valid IRI reference",
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
  "format": "iri"
}
"""

_VALIDATE_FORMATS = False

class Iri0Deserializer(DeserializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


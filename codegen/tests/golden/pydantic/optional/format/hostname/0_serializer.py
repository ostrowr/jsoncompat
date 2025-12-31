"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "hostname"
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
    "data": "www.example.com",
    "description": "a valid host name",
    "valid": true
  },
  {
    "data": "xn--4gbwdl.xn--wgbh1c",
    "description": "a valid punycoded IDN hostname",
    "valid": true
  },
  {
    "data": "-a-host-name-that-starts-with--",
    "description": "a host name starting with an illegal character",
    "valid": false
  },
  {
    "data": "not_a_valid_host_name",
    "description": "a host name containing illegal characters",
    "valid": false
  },
  {
    "data": "a-vvvvvvvvvvvvvvvveeeeeeeeeeeeeeeerrrrrrrrrrrrrrrryyyyyyyyyyyyyyyy-long-host-name-component",
    "description": "a host name with a component too long",
    "valid": false
  },
  {
    "data": "-hostname",
    "description": "starts with hyphen",
    "valid": false
  },
  {
    "data": "hostname-",
    "description": "ends with hyphen",
    "valid": false
  },
  {
    "data": "_hostname",
    "description": "starts with underscore",
    "valid": false
  },
  {
    "data": "hostname_",
    "description": "ends with underscore",
    "valid": false
  },
  {
    "data": "host_name",
    "description": "contains underscore",
    "valid": false
  },
  {
    "data": "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijk.com",
    "description": "maximum label length",
    "valid": true
  },
  {
    "data": "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijkl.com",
    "description": "exceeds maximum label length",
    "valid": false
  },
  {
    "data": "hostname",
    "description": "single label",
    "valid": true
  },
  {
    "data": "host-name",
    "description": "single label with hyphen",
    "valid": true
  },
  {
    "data": "h0stn4me",
    "description": "single label with digits",
    "valid": true
  },
  {
    "data": "1host",
    "description": "single label starting with digit",
    "valid": true
  },
  {
    "data": "hostnam3",
    "description": "single label ending with digit",
    "valid": true
  },
  {
    "data": "",
    "description": "empty string",
    "valid": false
  },
  {
    "data": ".",
    "description": "single dot",
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
  "format": "hostname"
}
"""

_VALIDATE_FORMATS = False

class Hostname0Serializer(SerializerRootModel):
    _validate_formats = _VALIDATE_FORMATS
    __json_schema__ = _JSON_SCHEMA
    root: Any


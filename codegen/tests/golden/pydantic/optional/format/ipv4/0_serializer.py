"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "ipv4"
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
    "data": "192.168.0.1",
    "description": "a valid IP address",
    "valid": true
  },
  {
    "data": "127.0.0.0.1",
    "description": "an IP address with too many components",
    "valid": false
  },
  {
    "data": "256.256.256.256",
    "description": "an IP address with out-of-range values",
    "valid": false
  },
  {
    "data": "127.0",
    "description": "an IP address without 4 components",
    "valid": false
  },
  {
    "data": "0x7f000001",
    "description": "an IP address as an integer",
    "valid": false
  },
  {
    "data": "2130706433",
    "description": "an IP address as an integer (decimal)",
    "valid": false
  },
  {
    "comment": "see https://sick.codes/universal-netmask-npm-package-used-by-270000-projects-vulnerable-to-octal-input-data-server-side-request-forgery-remote-file-inclusion-local-file-inclusion-and-more-cve-2021-28918/",
    "data": "087.10.0.1",
    "description": "invalid leading zeroes, as they are treated as octals",
    "valid": false
  },
  {
    "data": "87.10.0.1",
    "description": "value without leading zero is valid",
    "valid": true
  },
  {
    "data": "1২7.0.0.1",
    "description": "invalid non-ASCII '২' (a Bengali 2)",
    "valid": false
  },
  {
    "data": "192.168.1.0/24",
    "description": "netmask is not a part of ipv4 address",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ipv40Serializer(SerializerRootModel):
    root: Any


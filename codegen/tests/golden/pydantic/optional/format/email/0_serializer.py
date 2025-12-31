"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "email"
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
    "data": "joe.bloggs@example.com",
    "description": "a valid e-mail address",
    "valid": true
  },
  {
    "data": "2962",
    "description": "an invalid e-mail address",
    "valid": false
  },
  {
    "data": "te~st@example.com",
    "description": "tilde in local part is valid",
    "valid": true
  },
  {
    "data": "~test@example.com",
    "description": "tilde before local part is valid",
    "valid": true
  },
  {
    "data": "test~@example.com",
    "description": "tilde after local part is valid",
    "valid": true
  },
  {
    "data": "\"joe bloggs\"@example.com",
    "description": "a quoted string with a space in the local part is valid",
    "valid": true
  },
  {
    "data": "\"joe..bloggs\"@example.com",
    "description": "a quoted string with a double dot in the local part is valid",
    "valid": true
  },
  {
    "data": "\"joe@bloggs\"@example.com",
    "description": "a quoted string with a @ in the local part is valid",
    "valid": true
  },
  {
    "data": "joe.bloggs@[127.0.0.1]",
    "description": "an IPv4-address-literal after the @ is valid",
    "valid": true
  },
  {
    "data": "joe.bloggs@[IPv6:::1]",
    "description": "an IPv6-address-literal after the @ is valid",
    "valid": true
  },
  {
    "data": ".test@example.com",
    "description": "dot before local part is not valid",
    "valid": false
  },
  {
    "data": "test.@example.com",
    "description": "dot after local part is not valid",
    "valid": false
  },
  {
    "data": "te.s.t@example.com",
    "description": "two separated dots inside local part are valid",
    "valid": true
  },
  {
    "data": "te..st@example.com",
    "description": "two subsequent dots inside local part are not valid",
    "valid": false
  },
  {
    "data": "joe.bloggs@invalid=domain.com",
    "description": "an invalid domain",
    "valid": false
  },
  {
    "data": "joe.bloggs@[127.0.0.300]",
    "description": "an invalid IPv4-address-literal",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Email0Serializer(SerializerRootModel):
    root: Any


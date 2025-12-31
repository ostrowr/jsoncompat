"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "idn-email"
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
    "data": "실례@실례.테스트",
    "description": "a valid idn e-mail (example@example.test in Hangul)",
    "valid": true
  },
  {
    "data": "2962",
    "description": "an invalid idn e-mail address",
    "valid": false
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
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Idnemail0Serializer(SerializerRootModel):
    root: Any


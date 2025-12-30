"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "unknown"
}

Tests:
[
  {
    "data": 12,
    "description": "unknown formats ignore integers",
    "valid": true
  },
  {
    "data": 13.7,
    "description": "unknown formats ignore floats",
    "valid": true
  },
  {
    "data": {},
    "description": "unknown formats ignore objects",
    "valid": true
  },
  {
    "data": [],
    "description": "unknown formats ignore arrays",
    "valid": true
  },
  {
    "data": false,
    "description": "unknown formats ignore booleans",
    "valid": true
  },
  {
    "data": null,
    "description": "unknown formats ignore nulls",
    "valid": true
  },
  {
    "data": "string",
    "description": "unknown formats ignore strings",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Unknown0Serializer(SerializerRootModel):
    root: Any


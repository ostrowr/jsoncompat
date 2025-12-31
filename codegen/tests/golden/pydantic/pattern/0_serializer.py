"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "^a*$"
}

Tests:
[
  {
    "data": "aaa",
    "description": "a matching pattern is valid",
    "valid": true
  },
  {
    "data": "abc",
    "description": "a non-matching pattern is invalid",
    "valid": false
  },
  {
    "data": true,
    "description": "ignores booleans",
    "valid": true
  },
  {
    "data": 123,
    "description": "ignores integers",
    "valid": true
  },
  {
    "data": 1.0,
    "description": "ignores floats",
    "valid": true
  },
  {
    "data": {},
    "description": "ignores objects",
    "valid": true
  },
  {
    "data": [],
    "description": "ignores arrays",
    "valid": true
  },
  {
    "data": null,
    "description": "ignores null",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Pattern0Serializer(SerializerRootModel):
    root: Annotated[str, Field(pattern="^a*$")]


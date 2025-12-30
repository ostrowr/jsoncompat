"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "unevaluatedProperties": false
}

Tests:
[
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
    "data": [],
    "description": "ignores arrays",
    "valid": true
  },
  {
    "data": "foo",
    "description": "ignores strings",
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

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Unevaluatedproperties35Deserializer(DeserializerRootModel):
    root: Any


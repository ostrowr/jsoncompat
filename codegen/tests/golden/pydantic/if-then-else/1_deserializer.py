"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "then": {
    "const": 0
  }
}

Tests:
[
  {
    "data": 0,
    "description": "valid when valid against lone then",
    "valid": true
  },
  {
    "data": "hello",
    "description": "valid when invalid against lone then",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ifthenelse1Deserializer(DeserializerRootModel):
    root: Any


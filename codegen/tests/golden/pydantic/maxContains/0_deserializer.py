"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "maxContains": 1
}

Tests:
[
  {
    "data": [
      1
    ],
    "description": "one item valid against lone maxContains",
    "valid": true
  },
  {
    "data": [
      1,
      2
    ],
    "description": "two items still valid against lone maxContains",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Maxcontains0Deserializer(DeserializerRootModel):
    root: Any


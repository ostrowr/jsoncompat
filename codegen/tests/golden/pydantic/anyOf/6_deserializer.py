"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "type": "number"
    },
    {}
  ]
}

Tests:
[
  {
    "data": "foo",
    "description": "string is valid",
    "valid": true
  },
  {
    "data": 123,
    "description": "number is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Anyof6Deserializer(DeserializerRootModel):
    root: float | Any


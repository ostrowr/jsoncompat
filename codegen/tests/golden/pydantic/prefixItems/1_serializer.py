"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "prefixItems": [
    true,
    false
  ]
}

Tests:
[
  {
    "data": [
      1
    ],
    "description": "array with one item is valid",
    "valid": true
  },
  {
    "data": [
      1,
      "foo"
    ],
    "description": "array with two items is invalid",
    "valid": false
  },
  {
    "data": [],
    "description": "empty array is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Prefixitems1Serializer(SerializerRootModel):
    root: Any


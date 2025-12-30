"""
Schema:
true

Tests:
[
  {
    "data": 1,
    "description": "number is valid",
    "valid": true
  },
  {
    "data": "foo",
    "description": "string is valid",
    "valid": true
  },
  {
    "data": true,
    "description": "boolean true is valid",
    "valid": true
  },
  {
    "data": false,
    "description": "boolean false is valid",
    "valid": true
  },
  {
    "data": null,
    "description": "null is valid",
    "valid": true
  },
  {
    "data": {
      "foo": "bar"
    },
    "description": "object is valid",
    "valid": true
  },
  {
    "data": {},
    "description": "empty object is valid",
    "valid": true
  },
  {
    "data": [
      "foo"
    ],
    "description": "array is valid",
    "valid": true
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

class Booleanschema0Deserializer(DeserializerRootModel):
    root: Any


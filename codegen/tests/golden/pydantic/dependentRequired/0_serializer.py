"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "dependentRequired": {
    "bar": [
      "foo"
    ]
  }
}

Tests:
[
  {
    "data": {},
    "description": "neither",
    "valid": true
  },
  {
    "data": {
      "foo": 1
    },
    "description": "nondependant",
    "valid": true
  },
  {
    "data": {
      "bar": 2,
      "foo": 1
    },
    "description": "with dependency",
    "valid": true
  },
  {
    "data": {
      "bar": 2
    },
    "description": "missing dependency",
    "valid": false
  },
  {
    "data": [
      "bar"
    ],
    "description": "ignores arrays",
    "valid": true
  },
  {
    "data": "foobar",
    "description": "ignores strings",
    "valid": true
  },
  {
    "data": 12,
    "description": "ignores other non-objects",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Dependentrequired0Serializer(SerializerRootModel):
    root: Any


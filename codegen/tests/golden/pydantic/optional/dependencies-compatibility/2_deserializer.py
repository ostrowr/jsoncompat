"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "dependencies": {
    "quux": [
      "foo",
      "bar"
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
      "bar": 2,
      "foo": 1
    },
    "description": "nondependants",
    "valid": true
  },
  {
    "data": {
      "bar": 2,
      "foo": 1,
      "quux": 3
    },
    "description": "with dependencies",
    "valid": true
  },
  {
    "data": {
      "foo": 1,
      "quux": 2
    },
    "description": "missing dependency",
    "valid": false
  },
  {
    "data": {
      "bar": 1,
      "quux": 2
    },
    "description": "missing other dependency",
    "valid": false
  },
  {
    "data": {
      "quux": 1
    },
    "description": "missing both dependencies",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Dependenciescompatibility2Deserializer(DeserializerRootModel):
    root: Any


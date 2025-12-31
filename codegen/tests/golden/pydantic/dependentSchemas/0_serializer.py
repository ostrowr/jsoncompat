"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "dependentSchemas": {
    "bar": {
      "properties": {
        "bar": {
          "type": "integer"
        },
        "foo": {
          "type": "integer"
        }
      }
    }
  }
}

Tests:
[
  {
    "data": {
      "bar": 2,
      "foo": 1
    },
    "description": "valid",
    "valid": true
  },
  {
    "data": {
      "foo": "quux"
    },
    "description": "no dependency",
    "valid": true
  },
  {
    "data": {
      "bar": 2,
      "foo": "quux"
    },
    "description": "wrong type",
    "valid": false
  },
  {
    "data": {
      "bar": "quux",
      "foo": 2
    },
    "description": "wrong type other",
    "valid": false
  },
  {
    "data": {
      "bar": "quux",
      "foo": "quux"
    },
    "description": "wrong type both",
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

class Dependentschemas0Serializer(SerializerRootModel):
    root: Any


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "dependentSchemas": {
    "foo\tbar": {
      "minProperties": 4
    },
    "foo'bar": {
      "required": [
        "foo\"bar"
      ]
    }
  }
}

Tests:
[
  {
    "data": {
      "a": 2,
      "b": 3,
      "c": 4,
      "foo\tbar": 1
    },
    "description": "quoted tab",
    "valid": true
  },
  {
    "data": {
      "foo'bar": {
        "foo\"bar": 1
      }
    },
    "description": "quoted quote",
    "valid": false
  },
  {
    "data": {
      "a": 2,
      "foo\tbar": 1
    },
    "description": "quoted tab invalid under dependent schema",
    "valid": false
  },
  {
    "data": {
      "foo'bar": 1
    },
    "description": "quoted quote invalid under dependent schema",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Dependentschemas2Deserializer(DeserializerRootModel):
    root: Any


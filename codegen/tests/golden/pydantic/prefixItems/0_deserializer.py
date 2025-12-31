"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "prefixItems": [
    {
      "type": "integer"
    },
    {
      "type": "string"
    }
  ]
}

Tests:
[
  {
    "data": [
      1,
      "foo"
    ],
    "description": "correct types",
    "valid": true
  },
  {
    "data": [
      "foo",
      1
    ],
    "description": "wrong types",
    "valid": false
  },
  {
    "data": [
      1
    ],
    "description": "incomplete array of items",
    "valid": true
  },
  {
    "data": [
      1,
      "foo",
      true
    ],
    "description": "array with additional items",
    "valid": true
  },
  {
    "data": [],
    "description": "empty array",
    "valid": true
  },
  {
    "data": {
      "0": "invalid",
      "1": "valid",
      "length": 2
    },
    "description": "JavaScript pseudo-array is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Prefixitems0Deserializer(DeserializerRootModel):
    root: Any


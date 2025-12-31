"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "prefixItems": [
    {
      "type": "boolean"
    },
    {
      "type": "boolean"
    }
  ],
  "uniqueItems": false
}

Tests:
[
  {
    "data": [
      false,
      true
    ],
    "description": "[false, true] from items array is valid",
    "valid": true
  },
  {
    "data": [
      true,
      false
    ],
    "description": "[true, false] from items array is valid",
    "valid": true
  },
  {
    "data": [
      false,
      false
    ],
    "description": "[false, false] from items array is valid",
    "valid": true
  },
  {
    "data": [
      true,
      true
    ],
    "description": "[true, true] from items array is valid",
    "valid": true
  },
  {
    "data": [
      false,
      true,
      "foo",
      "bar"
    ],
    "description": "unique array extended from [false, true] is valid",
    "valid": true
  },
  {
    "data": [
      true,
      false,
      "foo",
      "bar"
    ],
    "description": "unique array extended from [true, false] is valid",
    "valid": true
  },
  {
    "data": [
      false,
      true,
      "foo",
      "foo"
    ],
    "description": "non-unique array extended from [false, true] is valid",
    "valid": true
  },
  {
    "data": [
      true,
      false,
      "foo",
      "foo"
    ],
    "description": "non-unique array extended from [true, false] is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Uniqueitems4Serializer(SerializerRootModel):
    root: Any


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "not": true
}

Tests:
[
  {
    "data": 1,
    "description": "number is invalid",
    "valid": false
  },
  {
    "data": "foo",
    "description": "string is invalid",
    "valid": false
  },
  {
    "data": true,
    "description": "boolean true is invalid",
    "valid": false
  },
  {
    "data": false,
    "description": "boolean false is invalid",
    "valid": false
  },
  {
    "data": null,
    "description": "null is invalid",
    "valid": false
  },
  {
    "data": {
      "foo": "bar"
    },
    "description": "object is invalid",
    "valid": false
  },
  {
    "data": {},
    "description": "empty object is invalid",
    "valid": false
  },
  {
    "data": [
      "foo"
    ],
    "description": "array is invalid",
    "valid": false
  },
  {
    "data": [],
    "description": "empty array is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Not5Deserializer(DeserializerRootModel):
    root: Any


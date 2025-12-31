"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "dependentSchemas": {
    "bar": false,
    "foo": true
  }
}

Tests:
[
  {
    "data": {
      "foo": 1
    },
    "description": "object with property having schema true is valid",
    "valid": true
  },
  {
    "data": {
      "bar": 2
    },
    "description": "object with property having schema false is invalid",
    "valid": false
  },
  {
    "data": {
      "bar": 2,
      "foo": 1
    },
    "description": "object with both properties is invalid",
    "valid": false
  },
  {
    "data": {},
    "description": "empty object is valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Dependentschemas1Serializer(SerializerRootModel):
    root: Any


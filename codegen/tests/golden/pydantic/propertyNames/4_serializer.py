"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "propertyNames": {
    "const": "foo"
  }
}

Tests:
[
  {
    "data": {
      "foo": 1
    },
    "description": "object with property foo is valid",
    "valid": true
  },
  {
    "data": {
      "bar": 1
    },
    "description": "object with any other property is invalid",
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

class Propertynames4Serializer(SerializerRootModel):
    root: Any


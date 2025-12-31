"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "propertyNames": {
    "maxLength": 3
  }
}

Tests:
[
  {
    "data": {
      "f": {},
      "foo": {}
    },
    "description": "all property names valid",
    "valid": true
  },
  {
    "data": {
      "foo": {},
      "foobar": {}
    },
    "description": "some property names invalid",
    "valid": false
  },
  {
    "data": {},
    "description": "object without properties is valid",
    "valid": true
  },
  {
    "data": [
      1,
      2,
      3,
      4
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

class Propertynames0Deserializer(DeserializerRootModel):
    root: Any


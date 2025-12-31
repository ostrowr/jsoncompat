"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "type": "number"
    },
    {}
  ]
}

Tests:
[
  {
    "data": "foo",
    "description": "one valid - valid",
    "valid": true
  },
  {
    "data": 123,
    "description": "both valid - invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Oneof7Serializer(SerializerRootModel):
    root: float | Any


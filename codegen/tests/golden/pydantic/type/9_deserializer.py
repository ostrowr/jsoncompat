"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": [
    "array",
    "object"
  ]
}

Tests:
[
  {
    "data": [
      1,
      2,
      3
    ],
    "description": "array is valid",
    "valid": true
  },
  {
    "data": {
      "foo": 123
    },
    "description": "object is valid",
    "valid": true
  },
  {
    "data": 123,
    "description": "number is invalid",
    "valid": false
  },
  {
    "data": "foo",
    "description": "string is invalid",
    "valid": false
  },
  {
    "data": null,
    "description": "null is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class ModelDeserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")

class Type9Deserializer(DeserializerRootModel):
    root: list[Any] | ModelDeserializer


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "patternProperties": {
    "b.*": false,
    "f.*": true
  }
}

Tests:
[
  {
    "data": {
      "foo": 1
    },
    "description": "object with property matching schema true is valid",
    "valid": true
  },
  {
    "data": {
      "bar": 2
    },
    "description": "object with property matching schema false is invalid",
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
    "data": {
      "foobar": 1
    },
    "description": "object with a property matching both true and false is invalid",
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

class Patternproperties3Serializer(SerializerRootModel):
    root: Any


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "array"
}

Tests:
[
  {
    "data": 1,
    "description": "an integer is not an array",
    "valid": false
  },
  {
    "data": 1.1,
    "description": "a float is not an array",
    "valid": false
  },
  {
    "data": "foo",
    "description": "a string is not an array",
    "valid": false
  },
  {
    "data": {},
    "description": "an object is not an array",
    "valid": false
  },
  {
    "data": [],
    "description": "an array is an array",
    "valid": true
  },
  {
    "data": true,
    "description": "a boolean is not an array",
    "valid": false
  },
  {
    "data": null,
    "description": "null is not an array",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Type4Deserializer(DeserializerRootModel):
    root: list[Any]


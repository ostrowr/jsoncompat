"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": [
    "integer",
    "string"
  ]
}

Tests:
[
  {
    "data": 1,
    "description": "an integer is valid",
    "valid": true
  },
  {
    "data": "foo",
    "description": "a string is valid",
    "valid": true
  },
  {
    "data": 1.1,
    "description": "a float is invalid",
    "valid": false
  },
  {
    "data": {},
    "description": "an object is invalid",
    "valid": false
  },
  {
    "data": [],
    "description": "an array is invalid",
    "valid": false
  },
  {
    "data": true,
    "description": "a boolean is invalid",
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

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Type7Serializer(SerializerRootModel):
    root: int | str


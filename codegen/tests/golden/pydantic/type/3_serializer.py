"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object"
}

Tests:
[
  {
    "data": 1,
    "description": "an integer is not an object",
    "valid": false
  },
  {
    "data": 1.1,
    "description": "a float is not an object",
    "valid": false
  },
  {
    "data": "foo",
    "description": "a string is not an object",
    "valid": false
  },
  {
    "data": {},
    "description": "an object is an object",
    "valid": true
  },
  {
    "data": [],
    "description": "an array is not an object",
    "valid": false
  },
  {
    "data": true,
    "description": "a boolean is not an object",
    "valid": false
  },
  {
    "data": null,
    "description": "null is not an object",
    "valid": false
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Type3Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")


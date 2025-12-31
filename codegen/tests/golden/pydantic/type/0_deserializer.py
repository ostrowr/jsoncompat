"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "integer"
}

Tests:
[
  {
    "data": 1,
    "description": "an integer is an integer",
    "valid": true
  },
  {
    "data": 1.0,
    "description": "a float with zero fractional part is an integer",
    "valid": true
  },
  {
    "data": 1.1,
    "description": "a float is not an integer",
    "valid": false
  },
  {
    "data": "foo",
    "description": "a string is not an integer",
    "valid": false
  },
  {
    "data": "1",
    "description": "a string is still not an integer, even if it looks like one",
    "valid": false
  },
  {
    "data": {},
    "description": "an object is not an integer",
    "valid": false
  },
  {
    "data": [],
    "description": "an array is not an integer",
    "valid": false
  },
  {
    "data": true,
    "description": "a boolean is not an integer",
    "valid": false
  },
  {
    "data": null,
    "description": "null is not an integer",
    "valid": false
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Type0Deserializer(DeserializerRootModel):
    root: int


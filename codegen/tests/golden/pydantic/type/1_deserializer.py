"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "number"
}

Tests:
[
  {
    "data": 1,
    "description": "an integer is a number",
    "valid": true
  },
  {
    "data": 1.0,
    "description": "a float with zero fractional part is a number (and an integer)",
    "valid": true
  },
  {
    "data": 1.1,
    "description": "a float is a number",
    "valid": true
  },
  {
    "data": "foo",
    "description": "a string is not a number",
    "valid": false
  },
  {
    "data": "1",
    "description": "a string is still not a number, even if it looks like one",
    "valid": false
  },
  {
    "data": {},
    "description": "an object is not a number",
    "valid": false
  },
  {
    "data": [],
    "description": "an array is not a number",
    "valid": false
  },
  {
    "data": true,
    "description": "a boolean is not a number",
    "valid": false
  },
  {
    "data": null,
    "description": "null is not a number",
    "valid": false
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Type1Deserializer(DeserializerRootModel):
    root: float


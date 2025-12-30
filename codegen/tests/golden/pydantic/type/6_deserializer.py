"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "null"
}

Tests:
[
  {
    "data": 1,
    "description": "an integer is not null",
    "valid": false
  },
  {
    "data": 1.1,
    "description": "a float is not null",
    "valid": false
  },
  {
    "data": 0,
    "description": "zero is not null",
    "valid": false
  },
  {
    "data": "foo",
    "description": "a string is not null",
    "valid": false
  },
  {
    "data": "",
    "description": "an empty string is not null",
    "valid": false
  },
  {
    "data": {},
    "description": "an object is not null",
    "valid": false
  },
  {
    "data": [],
    "description": "an array is not null",
    "valid": false
  },
  {
    "data": true,
    "description": "true is not null",
    "valid": false
  },
  {
    "data": false,
    "description": "false is not null",
    "valid": false
  },
  {
    "data": null,
    "description": "null is null",
    "valid": true
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Type6Deserializer(DeserializerRootModel):
    root: None


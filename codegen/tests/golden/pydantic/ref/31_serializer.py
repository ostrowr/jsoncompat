"""
Schema:
{
  "$defs": {
    "a": {
      "$id": "http://example.com/ref/absref/foobar.json",
      "type": "number"
    },
    "b": {
      "$id": "http://example.com/absref/foobar.json",
      "type": "string"
    }
  },
  "$id": "http://example.com/ref/absref.json",
  "$ref": "/absref/foobar.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": "foo",
    "description": "a string is valid",
    "valid": true
  },
  {
    "data": 12,
    "description": "an integer is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ref31Serializer(SerializerRootModel):
    root: Any


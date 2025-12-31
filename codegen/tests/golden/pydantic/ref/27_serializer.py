"""
Schema:
{
  "$defs": {
    "foo": {
      "$defs": {
        "bar": {
          "type": "string"
        }
      },
      "$id": "urn:uuid:deadbeef-4321-ffff-ffff-1234feebdaed",
      "$ref": "#/$defs/bar"
    }
  },
  "$ref": "urn:uuid:deadbeef-4321-ffff-ffff-1234feebdaed",
  "$schema": "https://json-schema.org/draft/2020-12/schema"
}

Tests:
[
  {
    "data": "bar",
    "description": "a string is valid",
    "valid": true
  },
  {
    "data": 12,
    "description": "a non-string is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ref27Serializer(SerializerRootModel):
    root: Any


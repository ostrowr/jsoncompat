"""
Schema:
{
  "$comment": "RFC 8141 §2.2",
  "$defs": {
    "bar": {
      "type": "string"
    }
  },
  "$id": "urn:example:1/406/47452/2",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "$ref": "#/$defs/bar"
    }
  }
}

Tests:
[
  {
    "data": {
      "foo": "bar"
    },
    "description": "a string is valid",
    "valid": true
  },
  {
    "data": {
      "foo": 12
    },
    "description": "a non-string is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Ref22Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    foo: Annotated[str | None, Field(default=None)]


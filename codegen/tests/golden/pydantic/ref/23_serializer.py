"""
Schema:
{
  "$comment": "RFC 8141 §2.3.1",
  "$defs": {
    "bar": {
      "type": "string"
    }
  },
  "$id": "urn:example:foo-bar-baz-qux?+CCResolve:cc=uk",
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

class Ref23Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    foo: Annotated[str | None, Field(default=None)]


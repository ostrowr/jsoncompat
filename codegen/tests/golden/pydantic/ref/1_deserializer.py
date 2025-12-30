"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar": {
      "$ref": "#/properties/foo"
    },
    "foo": {
      "type": "integer"
    }
  }
}

Tests:
[
  {
    "data": {
      "bar": 3
    },
    "description": "match",
    "valid": true
  },
  {
    "data": {
      "bar": true
    },
    "description": "mismatch",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Ref1Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Annotated[int | None, Field(default=None)]
    foo: Annotated[int | None, Field(default=None)]


"""
Schema:
{
  "$defs": {
    "bar": {
      "properties": {
        "bar": {
          "type": "string"
        }
      }
    }
  },
  "$ref": "#/$defs/bar",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "type": "string"
    }
  },
  "type": "object",
  "unevaluatedProperties": false
}

Tests:
[
  {
    "data": {
      "bar": "bar",
      "foo": "foo"
    },
    "description": "with no unevaluated properties",
    "valid": true
  },
  {
    "data": {
      "bar": "bar",
      "baz": "baz",
      "foo": "foo"
    },
    "description": "with unevaluated properties",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Unevaluatedproperties18Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Annotated[str | None, Field(default=None)]
    foo: Annotated[str | None, Field(default=None)]


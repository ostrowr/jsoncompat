"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "properties": {
        "foo": {
          "type": "string"
        }
      },
      "unevaluatedProperties": true
    }
  ],
  "type": "object",
  "unevaluatedProperties": false
}

Tests:
[
  {
    "data": {
      "foo": "foo"
    },
    "description": "with no nested unevaluated properties",
    "valid": true
  },
  {
    "data": {
      "bar": "bar",
      "foo": "foo"
    },
    "description": "with nested unevaluated properties",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Unevaluatedproperties24Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    foo: Annotated[str | None, Field(default=None)]


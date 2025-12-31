"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "allOf": [
    {
      "additionalProperties": true
    }
  ],
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
      "foo": "foo"
    },
    "description": "with no additional properties",
    "valid": true
  },
  {
    "data": {
      "bar": "bar",
      "foo": "foo"
    },
    "description": "with additional properties",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Unevaluatedproperties8Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    foo: Annotated[str | None, Field(default=None)]


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "foo": {
      "$ref": "#"
    }
  }
}

Tests:
[
  {
    "data": {
      "foo": false
    },
    "description": "match",
    "valid": true
  },
  {
    "data": {
      "foo": {
        "foo": false
      }
    },
    "description": "recursive match",
    "valid": true
  },
  {
    "data": {
      "bar": false
    },
    "description": "mismatch",
    "valid": false
  },
  {
    "data": {
      "foo": {
        "bar": false
      }
    },
    "description": "recursive mismatch",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Ref0Serializer(SerializerBase):
    model_config = ConfigDict(extra="forbid")
    foo: Annotated[Any | None, Field(default=None)]


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar": {
      "type": "string"
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
      "bar": "baz",
      "foo": 1
    },
    "description": "both properties present and valid is valid",
    "valid": true
  },
  {
    "data": {
      "bar": {},
      "foo": 1
    },
    "description": "one property invalid is invalid",
    "valid": false
  },
  {
    "data": {
      "bar": {},
      "foo": []
    },
    "description": "both properties invalid is invalid",
    "valid": false
  },
  {
    "data": {
      "quux": []
    },
    "description": "doesn't invalidate other properties",
    "valid": true
  },
  {
    "data": [],
    "description": "ignores arrays",
    "valid": true
  },
  {
    "data": 12,
    "description": "ignores other non-objects",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Properties0Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Annotated[str | None, Field(default=None)]
    foo: Annotated[int | None, Field(default=None)]


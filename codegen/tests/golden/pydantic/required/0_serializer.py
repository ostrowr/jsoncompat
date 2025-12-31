"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar": {},
    "foo": {}
  },
  "required": [
    "foo"
  ]
}

Tests:
[
  {
    "data": {
      "foo": 1
    },
    "description": "present required property is valid",
    "valid": true
  },
  {
    "data": {
      "bar": 1
    },
    "description": "non-present required property is invalid",
    "valid": false
  },
  {
    "data": [],
    "description": "ignores arrays",
    "valid": true
  },
  {
    "data": "",
    "description": "ignores strings",
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

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Required0Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Annotated[Any | None, Field(default=None)]
    foo: Annotated[Any, Field()]


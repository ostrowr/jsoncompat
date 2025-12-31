"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "not": {}
    }
  }
}

Tests:
[
  {
    "data": {
      "bar": 2,
      "foo": 1
    },
    "description": "property present",
    "valid": false
  },
  {
    "data": {
      "bar": 1,
      "baz": 2
    },
    "description": "property absent",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Not3Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    foo: Annotated[Any | None, Field(default=None)]


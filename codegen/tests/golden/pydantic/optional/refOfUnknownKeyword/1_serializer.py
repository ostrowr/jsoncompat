"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar": {
      "$ref": "#/properties/foo/unknown-keyword"
    },
    "foo": {
      "unknown-keyword": {
        "type": "integer"
      }
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

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Refofunknownkeyword1Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Annotated[int | None, Field(default=None)]
    foo: Annotated[Any | None, Field(default=None)]


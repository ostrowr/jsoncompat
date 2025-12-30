"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar": {
      "default": "bad",
      "minLength": 4,
      "type": "string"
    }
  }
}

Tests:
[
  {
    "data": {
      "bar": "good"
    },
    "description": "valid when property is specified",
    "valid": true
  },
  {
    "data": {},
    "description": "still valid when the invalid default is used",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Default1Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Annotated[str | None, Field(min_length=4, default="bad")]


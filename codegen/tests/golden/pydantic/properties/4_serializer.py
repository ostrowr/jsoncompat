"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {
      "type": "null"
    }
  }
}

Tests:
[
  {
    "data": {
      "foo": null
    },
    "description": "allows null values",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Properties4Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    foo: Annotated[None, Field(default=None)]


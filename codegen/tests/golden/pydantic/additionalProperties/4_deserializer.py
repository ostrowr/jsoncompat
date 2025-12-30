"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar": {},
    "foo": {}
  }
}

Tests:
[
  {
    "data": {
      "bar": 2,
      "foo": 1,
      "quux": true
    },
    "description": "additional properties are allowed",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Additionalproperties4Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Annotated[Any | None, Field(default=None)]
    foo: Annotated[Any | None, Field(default=None)]


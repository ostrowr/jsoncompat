"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo": {}
  },
  "required": []
}

Tests:
[
  {
    "data": {},
    "description": "property not required",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Required2Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    foo: Annotated[Any | None, Field(default=None)]


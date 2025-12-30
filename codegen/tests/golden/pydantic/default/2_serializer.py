"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "alpha": {
      "default": 5,
      "maximum": 3,
      "type": "number"
    }
  },
  "type": "object"
}

Tests:
[
  {
    "data": {
      "alpha": 1
    },
    "description": "an explicit property value is checked against maximum (passing)",
    "valid": true
  },
  {
    "data": {
      "alpha": 5
    },
    "description": "an explicit property value is checked against maximum (failing)",
    "valid": false
  },
  {
    "data": {},
    "description": "missing properties are not filled in with the default",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Default2Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    alpha: Annotated[float | None, Field(le=3.0, default=None)]


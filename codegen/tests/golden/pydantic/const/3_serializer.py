"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "const": null
}

Tests:
[
  {
    "data": null,
    "description": "null is valid",
    "valid": true
  },
  {
    "data": 0,
    "description": "not null is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Literal

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Const3Serializer(SerializerRootModel):
    root: Literal[None]


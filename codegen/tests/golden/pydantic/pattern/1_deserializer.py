"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "pattern": "a+"
}

Tests:
[
  {
    "data": "xxaayy",
    "description": "matches a substring",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Pattern1Deserializer(DeserializerRootModel):
    root: Annotated[str, Field(pattern="a+")]


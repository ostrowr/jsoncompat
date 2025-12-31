"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "format": "regex"
}

Tests:
[
  {
    "data": "\\a",
    "description": "when used as a pattern",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Ecmascriptregex0Deserializer(DeserializerRootModel):
    root: Any


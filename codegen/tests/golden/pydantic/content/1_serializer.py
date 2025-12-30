"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contentEncoding": "base64"
}

Tests:
[
  {
    "data": "eyJmb28iOiAiYmFyIn0K",
    "description": "a valid base64 string",
    "valid": true
  },
  {
    "data": "eyJmb28iOi%iYmFyIn0K",
    "description": "an invalid base64 string (% is not a valid character); validates true",
    "valid": true
  },
  {
    "data": 100,
    "description": "ignores non-strings",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Content1Serializer(SerializerRootModel):
    root: Any


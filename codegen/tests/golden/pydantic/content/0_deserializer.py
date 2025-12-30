"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contentMediaType": "application/json"
}

Tests:
[
  {
    "data": "{\"foo\": \"bar\"}",
    "description": "a valid JSON document",
    "valid": true
  },
  {
    "data": "{:}",
    "description": "an invalid JSON document; validates true",
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

class Content0Deserializer(DeserializerRootModel):
    root: Any


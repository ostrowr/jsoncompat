"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contentEncoding": "base64",
  "contentMediaType": "application/json"
}

Tests:
[
  {
    "data": "eyJmb28iOiAiYmFyIn0K",
    "description": "a valid base64-encoded JSON document",
    "valid": true
  },
  {
    "data": "ezp9Cg==",
    "description": "a validly-encoded invalid JSON document; validates true",
    "valid": true
  },
  {
    "data": "{}",
    "description": "an invalid base64 string that is valid JSON; validates true",
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

class Content2Serializer(SerializerRootModel):
    root: Any


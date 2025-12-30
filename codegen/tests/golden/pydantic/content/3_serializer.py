"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "contentEncoding": "base64",
  "contentMediaType": "application/json",
  "contentSchema": {
    "properties": {
      "foo": {
        "type": "string"
      }
    },
    "required": [
      "foo"
    ],
    "type": "object"
  }
}

Tests:
[
  {
    "data": "eyJmb28iOiAiYmFyIn0K",
    "description": "a valid base64-encoded JSON document",
    "valid": true
  },
  {
    "data": "eyJib28iOiAyMCwgImZvbyI6ICJiYXoifQ==",
    "description": "another valid base64-encoded JSON document",
    "valid": true
  },
  {
    "data": "eyJib28iOiAyMH0=",
    "description": "an invalid base64-encoded JSON document; validates true",
    "valid": true
  },
  {
    "data": "e30=",
    "description": "an empty object as a base64-encoded JSON document; validates true",
    "valid": true
  },
  {
    "data": "W10=",
    "description": "an empty array as a base64-encoded JSON document",
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

class Content3Serializer(SerializerRootModel):
    root: Any


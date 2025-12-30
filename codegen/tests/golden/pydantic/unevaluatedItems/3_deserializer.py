"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "type": "string"
  },
  "unevaluatedItems": false
}

Tests:
[
  {
    "data": [
      "foo",
      "bar"
    ],
    "description": "unevaluatedItems doesn't apply",
    "valid": true
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Unevaluateditems3Deserializer(DeserializerRootModel):
    root: list[str]


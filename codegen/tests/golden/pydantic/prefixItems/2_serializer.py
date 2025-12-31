"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "prefixItems": [
    {
      "type": "integer"
    }
  ]
}

Tests:
[
  {
    "data": [
      1,
      "foo",
      false
    ],
    "description": "only the first item is validated",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Prefixitems2Serializer(SerializerRootModel):
    root: Any


"""
Schema:
{
  "$id": "https://schema/using/format-assertion/false",
  "$schema": "http://localhost:1234/draft2020-12/format-assertion-false.json",
  "format": "ipv4"
}

Tests:
[
  {
    "data": "127.0.0.1",
    "description": "format-assertion: false: valid string",
    "valid": true
  },
  {
    "data": "not-an-ipv4",
    "description": "format-assertion: false: invalid string",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Formatassertion0Deserializer(DeserializerRootModel):
    root: Any


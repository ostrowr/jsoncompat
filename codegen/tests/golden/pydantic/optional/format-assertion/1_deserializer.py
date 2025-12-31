"""
Schema:
{
  "$id": "https://schema/using/format-assertion/true",
  "$schema": "http://localhost:1234/draft2020-12/format-assertion-true.json",
  "format": "ipv4"
}

Tests:
[
  {
    "data": "127.0.0.1",
    "description": "format-assertion: true: valid string",
    "valid": true
  },
  {
    "data": "not-an-ipv4",
    "description": "format-assertion: true: invalid string",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Formatassertion1Deserializer(DeserializerRootModel):
    root: Any


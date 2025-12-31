"""
Schema:
{
  "$id": "http://localhost:1234/draft2020-12/",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "$id": "baseUriChange/",
    "items": {
      "$ref": "folderInteger.json"
    }
  }
}

Tests:
[
  {
    "data": [
      [
        1
      ]
    ],
    "description": "base URI change ref valid",
    "valid": true
  },
  {
    "data": [
      [
        "a"
      ]
    ],
    "description": "base URI change ref invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Refremote4Deserializer(DeserializerRootModel):
    root: list[list[Any]]


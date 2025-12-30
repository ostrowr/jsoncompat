"""
Schema:
{
  "$schema": "http://localhost:1234/draft2020-12/metaschema-optional-vocabulary.json",
  "type": "number"
}

Tests:
[
  {
    "data": "foobar",
    "description": "string value",
    "valid": false
  },
  {
    "data": 20,
    "description": "number value",
    "valid": true
  }
]
"""

from __future__ import annotations

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Vocabulary1Deserializer(DeserializerRootModel):
    root: float


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "dependencies": {
    "foo\nbar": [
      "foo\rbar"
    ],
    "foo\"bar": [
      "foo'bar"
    ]
  }
}

Tests:
[
  {
    "data": {
      "foo\nbar": 1,
      "foo\rbar": 2
    },
    "description": "CRLF",
    "valid": true
  },
  {
    "data": {
      "foo\"bar": 2,
      "foo'bar": 1
    },
    "description": "quoted quotes",
    "valid": true
  },
  {
    "data": {
      "foo": 2,
      "foo\nbar": 1
    },
    "description": "CRLF missing dependent",
    "valid": false
  },
  {
    "data": {
      "foo\"bar": 2
    },
    "description": "quoted quotes missing dependent",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Dependenciescompatibility3Deserializer(DeserializerRootModel):
    root: Any


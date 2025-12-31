"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "patternProperties": {
    "X_": {
      "type": "string"
    },
    "[0-9]{2,}": {
      "type": "boolean"
    }
  }
}

Tests:
[
  {
    "data": {
      "answer 1": "42"
    },
    "description": "non recognized members are ignored",
    "valid": true
  },
  {
    "data": {
      "a31b": null
    },
    "description": "recognized members are accounted for",
    "valid": false
  },
  {
    "data": {
      "a_x_3": 3
    },
    "description": "regexes are case sensitive",
    "valid": true
  },
  {
    "data": {
      "a_X_3": 3
    },
    "description": "regexes are case sensitive, 2",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Any

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Patternproperties2Serializer(SerializerRootModel):
    root: Any


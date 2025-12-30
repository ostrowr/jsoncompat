"""
Schema:
{
  "$defs": {
    "percent%field": {
      "type": "integer"
    },
    "slash/field": {
      "type": "integer"
    },
    "tilde~field": {
      "type": "integer"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "percent": {
      "$ref": "#/$defs/percent%25field"
    },
    "slash": {
      "$ref": "#/$defs/slash~1field"
    },
    "tilde": {
      "$ref": "#/$defs/tilde~0field"
    }
  }
}

Tests:
[
  {
    "data": {
      "slash": "aoeu"
    },
    "description": "slash invalid",
    "valid": false
  },
  {
    "data": {
      "tilde": "aoeu"
    },
    "description": "tilde invalid",
    "valid": false
  },
  {
    "data": {
      "percent": "aoeu"
    },
    "description": "percent invalid",
    "valid": false
  },
  {
    "data": {
      "slash": 123
    },
    "description": "slash valid",
    "valid": true
  },
  {
    "data": {
      "tilde": 123
    },
    "description": "tilde valid",
    "valid": true
  },
  {
    "data": {
      "percent": 123
    },
    "description": "percent valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Ref3Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    percent: Annotated[int | None, Field(default=None)]
    slash: Annotated[int | None, Field(default=None)]
    tilde: Annotated[int | None, Field(default=None)]


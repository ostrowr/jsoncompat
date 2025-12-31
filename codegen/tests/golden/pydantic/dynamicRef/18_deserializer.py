"""
Schema:
{
  "$defs": {
    "false": false,
    "true": true
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "false": {
      "$dynamicRef": "#/$defs/false"
    },
    "true": {
      "$dynamicRef": "#/$defs/true"
    }
  }
}

Tests:
[
  {
    "data": {
      "true": 1
    },
    "description": "follow $dynamicRef to a true schema",
    "valid": true
  },
  {
    "data": {
      "false": 1
    },
    "description": "follow $dynamicRef to a false schema",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Dynamicref18Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    false: Annotated[Any | None, Field(default=None)]
    true: Annotated[Any | None, Field(default=None)]


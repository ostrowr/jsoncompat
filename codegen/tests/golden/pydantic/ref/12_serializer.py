"""
Schema:
{
  "$defs": {
    "foo\"bar": {
      "type": "number"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "foo\"bar": {
      "$ref": "#/$defs/foo%22bar"
    }
  }
}

Tests:
[
  {
    "data": {
      "foo\"bar": 1
    },
    "description": "object with numbers is valid",
    "valid": true
  },
  {
    "data": {
      "foo\"bar": "1"
    },
    "description": "object with strings is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Ref12Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    foo_bar: Annotated[float | None, Field(alias="foo\"bar", default=None)]


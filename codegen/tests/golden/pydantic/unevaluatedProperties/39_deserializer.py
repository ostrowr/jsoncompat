"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "dependentSchemas": {
    "foo": {},
    "foo2": {
      "properties": {
        "bar": {}
      }
    }
  },
  "properties": {
    "foo2": {}
  },
  "unevaluatedProperties": false
}

Tests:
[
  {
    "data": {
      "foo": ""
    },
    "description": "unevaluatedProperties doesn't consider dependentSchemas",
    "valid": false
  },
  {
    "data": {
      "bar": ""
    },
    "description": "unevaluatedProperties doesn't see bar when foo2 is absent",
    "valid": false
  },
  {
    "data": {
      "bar": "",
      "foo2": ""
    },
    "description": "unevaluatedProperties sees bar when foo2 is present",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Unevaluatedproperties39Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    foo2: Annotated[Any | None, Field(default=None)]


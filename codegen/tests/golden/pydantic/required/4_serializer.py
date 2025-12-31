"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "required": [
    "__proto__",
    "toString",
    "constructor"
  ]
}

Tests:
[
  {
    "data": [],
    "description": "ignores arrays",
    "valid": true
  },
  {
    "data": 12,
    "description": "ignores other non-objects",
    "valid": true
  },
  {
    "data": {},
    "description": "none of the properties mentioned",
    "valid": false
  },
  {
    "data": {
      "__proto__": "foo"
    },
    "description": "__proto__ present",
    "valid": false
  },
  {
    "data": {
      "toString": {
        "length": 37
      }
    },
    "description": "toString present",
    "valid": false
  },
  {
    "data": {
      "constructor": {
        "length": 37
      }
    },
    "description": "constructor present",
    "valid": false
  },
  {
    "data": {
      "__proto__": 12,
      "constructor": 37,
      "toString": {
        "length": "foo"
      }
    },
    "description": "all present",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Required4Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    proto: Annotated[Any, Field(alias="__proto__")]
    constructor: Annotated[Any, Field()]
    to_string: Annotated[Any, Field(alias="toString")]


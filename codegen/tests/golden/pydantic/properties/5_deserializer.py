"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "__proto__": {
      "type": "number"
    },
    "constructor": {
      "type": "number"
    },
    "toString": {
      "properties": {
        "length": {
          "type": "string"
        }
      }
    }
  }
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
    "valid": true
  },
  {
    "data": {
      "__proto__": "foo"
    },
    "description": "__proto__ not valid",
    "valid": false
  },
  {
    "data": {
      "toString": {
        "length": 37
      }
    },
    "description": "toString not valid",
    "valid": false
  },
  {
    "data": {
      "constructor": {
        "length": 37
      }
    },
    "description": "constructor not valid",
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
    "description": "all present and valid",
    "valid": true
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class ModelDeserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    length: Annotated[str | None, Field(default=None)]

class Properties5Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    proto: Annotated[float | None, Field(alias="__proto__", default=None)]
    constructor: Annotated[float | None, Field(default=None)]
    to_string: Annotated[ModelDeserializer | None, Field(alias="toString", default=None)]


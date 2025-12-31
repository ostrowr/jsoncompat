"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": {
    "type": "integer"
  },
  "patternProperties": {
    "f.o": {
      "minItems": 2
    }
  },
  "properties": {
    "bar": {
      "type": "array"
    },
    "foo": {
      "maxItems": 3,
      "type": "array"
    }
  }
}

Tests:
[
  {
    "data": {
      "foo": [
        1,
        2
      ]
    },
    "description": "property validates property",
    "valid": true
  },
  {
    "data": {
      "foo": [
        1,
        2,
        3,
        4
      ]
    },
    "description": "property invalidates property",
    "valid": false
  },
  {
    "data": {
      "foo": []
    },
    "description": "patternProperty invalidates property",
    "valid": false
  },
  {
    "data": {
      "fxo": [
        1,
        2
      ]
    },
    "description": "patternProperty validates nonproperty",
    "valid": true
  },
  {
    "data": {
      "fxo": []
    },
    "description": "patternProperty invalidates nonproperty",
    "valid": false
  },
  {
    "data": {
      "bar": []
    },
    "description": "additionalProperty ignores property",
    "valid": true
  },
  {
    "data": {
      "quux": 3
    },
    "description": "additionalProperty validates others",
    "valid": true
  },
  {
    "data": {
      "quux": "foo"
    },
    "description": "additionalProperty invalidates others",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Any

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Properties1Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    __pydantic_extra__: dict[str, int]
    bar: Annotated[list[Any] | None, Field(default=None)]
    foo: Annotated[list[Any] | None, Field(max_length=3, default=None)]


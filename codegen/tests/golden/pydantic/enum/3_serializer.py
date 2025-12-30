"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "bar": {
      "enum": [
        "bar"
      ]
    },
    "foo": {
      "enum": [
        "foo"
      ]
    }
  },
  "required": [
    "bar"
  ],
  "type": "object"
}

Tests:
[
  {
    "data": {
      "bar": "bar",
      "foo": "foo"
    },
    "description": "both properties are valid",
    "valid": true
  },
  {
    "data": {
      "bar": "bar",
      "foo": "foot"
    },
    "description": "wrong foo value",
    "valid": false
  },
  {
    "data": {
      "bar": "bart",
      "foo": "foo"
    },
    "description": "wrong bar value",
    "valid": false
  },
  {
    "data": {
      "bar": "bar"
    },
    "description": "missing optional property is valid",
    "valid": true
  },
  {
    "data": {
      "foo": "foo"
    },
    "description": "missing required property is invalid",
    "valid": false
  },
  {
    "data": {},
    "description": "missing all properties is invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated, Literal

from json_schema_codegen_base import DeserializerBase, SerializerBase
from pydantic import ConfigDict, Field

class Enum3Serializer(SerializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Annotated[Literal["bar"], Field()]
    foo: Annotated[Literal["foo"] | None, Field(default=None)]


"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "properties": {
        "bar": {
          "type": "integer"
        }
      },
      "required": [
        "bar"
      ]
    },
    {
      "properties": {
        "foo": {
          "type": "string"
        }
      },
      "required": [
        "foo"
      ]
    }
  ]
}

Tests:
[
  {
    "data": {
      "bar": 2
    },
    "description": "first anyOf valid (complex)",
    "valid": true
  },
  {
    "data": {
      "foo": "baz"
    },
    "description": "second anyOf valid (complex)",
    "valid": true
  },
  {
    "data": {
      "bar": 2,
      "foo": "baz"
    },
    "description": "both anyOf valid (complex)",
    "valid": true
  },
  {
    "data": {
      "bar": "quux",
      "foo": 2
    },
    "description": "neither anyOf valid (complex)",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class ModelDeserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    bar: Annotated[int, Field()]

class Model2Deserializer(DeserializerBase):
    model_config = ConfigDict(extra="allow")
    foo: Annotated[str, Field()]

class Anyof5Deserializer(DeserializerRootModel):
    root: ModelDeserializer | Model2Deserializer


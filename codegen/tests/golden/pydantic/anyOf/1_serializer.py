"""
Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "maxLength": 2
    },
    {
      "minLength": 4
    }
  ],
  "type": "string"
}

Tests:
[
  {
    "data": 3,
    "description": "mismatch base schema",
    "valid": false
  },
  {
    "data": "foobar",
    "description": "one anyOf valid",
    "valid": true
  },
  {
    "data": "foo",
    "description": "both anyOf invalid",
    "valid": false
  }
]
"""

from __future__ import annotations

from typing import Annotated

from json_schema_codegen_base import DeserializerBase, DeserializerRootModel, SerializerBase, SerializerRootModel
from pydantic import ConfigDict, Field

class Anyof1Serializer(SerializerRootModel):
    root: Annotated[str, Field(max_length=2)] | Annotated[str, Field(min_length=4)]

